use crate::browser::create_browser;
use crate::error::AuthError;
use crate::handlers::page_detection::{
    get_azure_error_details, is_azure_error_page, is_incorrect_password_visible,
    is_invalid_username_visible, is_username_warning_visible,
};
use crate::handlers::{auth_handlers::*, mfa_handlers::*, page_detection::is_input_visible};
use crate::utils::{CancellationToken, CredentialsProvider};
use headless_chrome::{Browser, Tab};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

/// Closes all browser tabs so Chrome exits gracefully and flushes
/// cookies/session data to the profile directory on disk.
/// Without this, Browser::drop sends SIGKILL and cookies are lost.
fn close_all_tabs(browser: &Browser) {
    // If we can't get the lock quickly, it's likely something is wrong/stuck.
    if let Ok(tabs) = browser.get_tabs().lock() {
        for tab in tabs.iter() {
            // We don't want to hang here if the browser is already dead.
            // tab.close(true) sends a message and waits for a response.
            let _ = tab.close(true);
        }
    }
    sleep(Duration::from_millis(200));
}

/// Extracts DSID cookie from the browser.
fn poll_dsid(tab: &Tab, domain: &str) -> anyhow::Result<Option<String>> {
    let cookies = tab
        .get_cookies()
        .map_err(|e| anyhow::anyhow!("Browser error: {}", e))?;
    if let Some(cookie) = cookies
        .iter()
        .find(|c| c.name == "DSID" && c.domain.contains(domain))
    {
        return Ok(Some(cookie.value.clone()));
    }

    Ok(None)
}

/// Attempts to handle the current page state.
fn try_handle_page(
    tab: &Tab,
    handled: &mut HashSet<&'static str>,
    email: Option<&String>,
    provider: &dyn CredentialsProvider,
    cancel_token: Option<&CancellationToken>,
) -> anyhow::Result<(bool, bool)> {
    // Returns (handler_matched, is_mfa_handler)
    if !handled.contains("pick_account") && handle_pick_account(tab)? {
        handled.insert("pick_account");
        return Ok((true, false));
    }

    if !handled.contains("session_conflict") && handle_session_conflict(tab)? {
        handled.insert("session_conflict");
        return Ok((true, false));
    }

    // Check for Azure AD error pages (ConvergedError)
    // This must be checked early as it indicates a fatal authentication error
    if is_azure_error_page(tab)? {
        handled.insert("azure_error");
        let error_msg = get_azure_error_details(tab)?;
        log::warn!("[!] Azure AD error page detected: {}", error_msg);
        return Err(AuthError::AuthenticationFailed {
            reason: error_msg,
            suggest_manual_mode: false, // This is often a simple credential error
            suggest_clear_cache: true,
        }
        .into());
    }

    if is_invalid_username_visible(tab)? {
        handled.insert("invalid_username");
        return Err(AuthError::InvalidUsername {
            message: "The username you entered may be incorrect or the account does not exist."
                .to_string(),
        }
        .into());
    }

    // Check for username warning (softer error that suggests potential typo)
    if let Some(warning_text) = is_username_warning_visible(tab)? {
        handled.insert("username_warning");
        return Err(AuthError::UsernameWarning {
            warning_text: warning_text.trim().to_string(),
        }
        .into());
    }

    if is_incorrect_password_visible(tab)? {
        handled.insert("incorrect_password");
        log::warn!("[!] Incorrect password detected");
        return Err(AuthError::IncorrectPassword {
            message: "Your account or password is incorrect.".to_string(),
        }
        .into());
    }

    if handle_remote_ngc_denied_next(tab)? {
        handled.remove("ngc_push");
        return Ok((true, false));
    }

    if !handled.contains("username") && is_input_visible(tab, "input[name=\"loginfmt\"]")? {
        fill_on_screen_and_click(
            tab,
            "input[name=\"loginfmt\"]",
            "Username (email): ",
            "#idSIButton9",
            false,
            email,
            provider,
        )?;
        handled.insert("username");
        return Ok((true, false));
    }

    if !handled.contains("ngc_error_use_password") && handle_ngc_error_use_password(tab, handled)? {
        handled.insert("ngc_error_use_password");
        return Ok((true, false));
    }

    if !handled.contains("use_app_instead") && handle_use_app_instead(tab)? {
        handled.insert("use_app_instead");
        return Ok((true, false));
    }

    if !handled.contains("ngc_push") && handle_authenticator_ngc_push(tab, provider, cancel_token)?
    {
        handled.insert("ngc_push");
        return Ok((true, true)); // MFA handler
    }

    if !handled.contains("password") && is_input_visible(tab, "input[name=\"passwd\"]")? {
        fill_on_screen_and_click(
            tab,
            "input[name=\"passwd\"]",
            "KU Password: ",
            "#idSIButton9",
            true,
            None,
            provider,
        )?;
        handled.insert("password");
        return Ok((true, false));
    }

    if !handled.contains("kmsi") && click_kmsi_if_present(tab)? {
        handled.insert("kmsi");
        return Ok((true, false));
    }

    if !handled.contains("push") && handle_authenticator_push_approval(tab, provider, cancel_token)?
    {
        handled.insert("push");
        return Ok((true, true)); // MFA handler
    }

    if !handled.contains("verification_code") && handle_verification_code_choice(tab)? {
        handled.insert("verification_code");
        return Ok((true, false));
    }

    // Handle OTP/code entry pages (SMS, email, TOTP from authenticator app).
    // Must run before detect_generic_error, as the code-entry page contains instructional
    // text in aria-live regions that would otherwise trigger a false-positive error.
    if !handled.contains("otp_entry") && handle_otp_entry(tab, provider)? {
        handled.insert("otp_entry");
        return Ok((true, false));
    }

    // Generic fallback detection for unexpected errors or page states
    // This catches scenarios we haven't explicitly coded for
    if !handled.contains("generic_error") {
        if let Some(error_msg) = crate::handlers::generic_detection::detect_generic_error(tab)? {
            handled.insert("generic_error");
            log::warn!("[!] Generic error detected: {}", error_msg);
            return Err(AuthError::AuthenticationFailed {
                reason: format!("Unexpected error encountered:\n\n{}", error_msg),
                suggest_manual_mode: true,
                suggest_clear_cache: true,
            }
            .into());
        }
    }

    if !handled.contains("unexpected_state") {
        if let Some(state_msg) =
            crate::handlers::generic_detection::detect_unexpected_page_state(tab)?
        {
            handled.insert("unexpected_state");
            log::warn!("[!] Unexpected page state: {}", state_msg);
            return Err(AuthError::AuthenticationFailed {
                reason: format!("Unexpected page state:\n\n{}", state_msg),
                suggest_manual_mode: true,
                suggest_clear_cache: false,
            }
            .into());
        }
    }

    Ok((false, false))
}

/// Main function to run login process and retrieve DSID.
#[allow(clippy::too_many_arguments)]
pub fn run_login_and_get_dsid(
    headless: bool,
    url: &str,
    domain: &str,
    user_agent: &str,
    no_auto_login: bool,
    email: Option<String>,
    provider: &dyn CredentialsProvider,
    cancel_token: Option<CancellationToken>,
    browser_pid_out: Option<Arc<Mutex<Option<u32>>>>,
) -> anyhow::Result<String> {
    const MAX_RETRIES: usize = 20;
    const STUCK_THRESHOLD: usize = 8; // ~3.2 seconds without progress before considering stuck
    const MAX_RESETS: usize = 2; // maximum page resets allowed

    let browser = match create_browser(user_agent, headless, no_auto_login) {
        Ok(b) => b,
        Err(e) => {
            return Err(AuthError::BrowserError {
                message: format!("Failed to create browser: {}", e),
            }
            .into())
        }
    };

    // Store the browser PID so it can be force-killed on cancel
    if let Some(ref pid_holder) = browser_pid_out {
        if let Some(pid) = browser.get_process_id() {
            *pid_holder.lock().unwrap() = Some(pid);
        }
    }

    // Try to get the first tab, if it's not there yet, create one.
    // This avoids leaving an empty "New Tab" or "Home Page" open.
    let tab = {
        let mut initial_tab = None;
        for _ in 0..10 {
            if let Ok(tabs) = browser.get_tabs().lock() {
                if let Some(t) = tabs.first() {
                    initial_tab = Some(std::sync::Arc::clone(t));
                    break;
                }
            }
            sleep(Duration::from_millis(200));
        }
        match initial_tab {
            Some(t) => t,
            None => browser.new_tab()?,
        }
    };

    tab.set_default_timeout(Duration::from_secs(30));

    // Navigate to the target URL and wait for the page to load.
    log::info!("[*] Navigating to: {}", url);
    tab.navigate_to(url)?;

    if let Err(e) = tab.wait_until_navigated() {
        log::warn!(
            "[!] Initial navigation wait timed out: {}, continuing...",
            e
        );
    }

    let mut handled: HashSet<&'static str> = HashSet::new();
    let mut last_url = "".to_string();
    let mut retries = 0;
    let mut reset_count = 0;
    let mut is_in_mfa_wait;

    loop {
        if let Some(token) = &cancel_token {
            if token.is_cancelled() {
                log::info!("[!] Cancellation requested, closing browser.");
                close_all_tabs(&browser);
                return Err(AuthError::Cancelled.into());
            }
        }

        // Use poll_dsid as a heartbeat too. If it fails, the browser is likely gone.
        let dsid_result = poll_dsid(&tab, domain);
        match dsid_result {
            Ok(Some(dsid)) => {
                log::info!("[✓] Found valid DSID, quitting.");
                close_all_tabs(&browser);
                return Ok(dsid);
            }
            Ok(None) => {} // Keep going
            Err(e) => {
                log::warn!("[!] Browser heartbeat lost (manual close?): {}", e);
                return Err(AuthError::BrowserError {
                    message: format!("Browser connection lost: {}", e),
                }
                .into());
            }
        }

        let current_url = tab.get_url();
        if current_url != last_url {
            log::info!("[*] Page: {}", current_url);
            last_url = current_url;
            retries = 0;
            handled.clear(); // Allow re-authentication if Microsoft loops back
        }

        if !no_auto_login {
            // Page guard: dismiss prompts when the page changes underneath.
            // Microsoft login is an SPA (URL constant), so the actual input
            // watcher is injected by fill_on_screen_and_click / handle_otp_entry
            // right before they block on a prompt. They set
            // window.__kuvpn_input_gone = true when their input disappears.
            // Here we just reset the flag and wire up the guard to read it.
            tab.evaluate("window.__kuvpn_input_gone = false;", false).ok();

            let tab_for_guard = std::sync::Arc::clone(&tab);
            let guard_url = tab.get_url();
            provider.set_page_guard(Box::new(move || {
                if tab_for_guard.get_url() != guard_url {
                    return false;
                }
                let gone = tab_for_guard
                    .evaluate("window.__kuvpn_input_gone === true", false)
                    .ok()
                    .and_then(|r| r.value)
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true); // eval failure → assume page changed
                !gone
            }));

            // try_handle_page can also fail if browser is closed
            match try_handle_page(
                &tab,
                &mut handled,
                email.as_ref(),
                provider,
                cancel_token.as_ref(),
            ) {
                Ok((true, is_mfa)) => {
                    retries = 0;
                    is_in_mfa_wait = is_mfa;
                }
                Ok((false, _)) => {
                    retries += 1;
                    is_in_mfa_wait = false;
                }
                Err(e) => {
                    log::warn!("[!] Handler error: {}", e);
                    // Close browser tabs before returning error
                    close_all_tabs(&browser);
                    return Err(e);
                }
            }

            provider.clear_page_guard();

            // Page reset logic for stuck automation (only in Full Auto mode)
            if retries > STUCK_THRESHOLD && !is_in_mfa_wait {
                reset_count += 1;

                if reset_count > MAX_RESETS {
                    close_all_tabs(&browser);
                    return Err(AuthError::AuthenticationFailed {
                        reason: format!(
                            "Full Auto mode unable to complete login after {} page resets. \
                            The authentication flow may have changed or network issues occurred.",
                            MAX_RESETS
                        ),
                        suggest_manual_mode: true,
                        suggest_clear_cache: true,
                    }
                    .into());
                }

                log::warn!(
                    "[!] Authentication stuck (no progress for {}s). Resetting page... (attempt {}/{})",
                    (STUCK_THRESHOLD * 400) / 1000,
                    reset_count,
                    MAX_RESETS
                );

                // Log current page state to help diagnose the issue
                if let Err(e) = crate::handlers::generic_detection::log_page_state(&tab) {
                    log::debug!("[!] Failed to log page state: {}", e);
                }

                // Navigate back to initial login URL
                if let Err(e) = tab.navigate_to(url) {
                    log::warn!(
                        "[!] Reset navigation failed: {}, continuing with current page",
                        e
                    );
                } else {
                    let _ = tab.wait_until_navigated();
                    handled.clear(); // Clear handler history to allow re-triggering
                    retries = 0;
                    last_url = String::new();
                }
            }

            if retries > MAX_RETRIES {
                close_all_tabs(&browser);
                return Err(AuthError::AuthenticationFailed {
                    reason: format!(
                        "Could not find a handler for the current page after {} retries: {}",
                        MAX_RETRIES, last_url
                    ),
                    suggest_manual_mode: true,
                    suggest_clear_cache: false,
                }
                .into());
            }
        }

        sleep(Duration::from_millis(400));
    }
}
