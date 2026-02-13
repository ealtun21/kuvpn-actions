use crate::browser::create_browser;
use crate::handlers::page_detection::{is_incorrect_password_visible, is_invalid_username_visible};
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
    let cookies = tab.get_cookies().map_err(|e| anyhow::anyhow!("Browser error: {}", e))?;
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
) -> anyhow::Result<bool> {
    if !handled.contains("pick_account") && handle_pick_account(tab)? {
        handled.insert("pick_account");
        return Ok(true);
    }

    if !handled.contains("session_conflict") && handle_session_conflict(tab)? {
        handled.insert("session_conflict");
        return Ok(true);
    }

    if is_invalid_username_visible(tab)? {
        handled.insert("invalid_username");
        return Err(anyhow::anyhow!(
            "Invalid username or account not found, Please re-run the program and try again."
        ));
    }

    if is_incorrect_password_visible(tab)? {
        handled.remove("password");
        return Ok(true);
    }

    if handle_remote_ngc_denied_next(tab)? {
        handled.remove("ngc_push");
        return Ok(true);
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
        return Ok(true);
    }

    if !handled.contains("ngc_error_use_password") && handle_ngc_error_use_password(tab, handled)? {
        handled.insert("ngc_error_use_password");
        return Ok(true);
    }

    if !handled.contains("use_app_instead") && handle_use_app_instead(tab)? {
        handled.insert("use_app_instead");
        return Ok(true);
    }

    if !handled.contains("ngc_push") && handle_authenticator_ngc_push(tab, provider, cancel_token)? {
        handled.insert("ngc_push");
        return Ok(true);
    }

    if !handled.contains("password") && is_input_visible(tab, "input[name=\"passwd\"]")? {
        fill_on_screen_and_click(
            tab,
            "input[name=\"passwd\"]",
            "Password: ",
            "#idSIButton9",
            true,
            None,
            provider,
        )?;
        handled.insert("password");
        return Ok(true);
    }

    if !handled.contains("kmsi") && click_kmsi_if_present(tab)? {
        handled.insert("kmsi");
        return Ok(true);
    }

    if !handled.contains("push") && handle_authenticator_push_approval(tab, provider, cancel_token)? {
        handled.insert("push");
        return Ok(true);
    }

    if !handled.contains("verification_code") && handle_verification_code_choice(tab)? {
        handled.insert("verification_code");
        return Ok(true);
    }

    Ok(false)
}

/// Main function to run login process and retrieve DSID.
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
    const MAX_RETRIES: usize = 10;

    let browser = match create_browser(user_agent, headless, no_auto_login) {
        Ok(b) => b,
        Err(e) => return Err(anyhow::anyhow!(format!("Failed to create browser: {e}"))),
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
        log::warn!("[!] Initial navigation wait timed out: {}, continuing...", e);
    }

    let mut handled: HashSet<&'static str> = HashSet::new();
    let mut last_url = "".to_string();
    let mut retries = 0;

    loop {
        if let Some(token) = &cancel_token {
            if token.is_cancelled() {
                log::info!("[!] Cancellation requested, closing browser.");
                close_all_tabs(&browser);
                return Err(anyhow::anyhow!("Operation cancelled by user"));
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
                return Err(e);
            }
        }

        let current_url = tab.get_url();
        if current_url != last_url {
            log::info!("[*] Page: {}", current_url);
            last_url = current_url;
            retries = 0;
        }

        if !no_auto_login {
            // try_handle_page can also fail if browser is closed
            match try_handle_page(&tab, &mut handled, email.as_ref(), provider, cancel_token.as_ref()) {
                Ok(true) => {
                    retries = 0;
                }
                Ok(false) => {
                    retries += 1;
                }
                Err(e) => {
                    log::warn!("[!] Handler error: {}", e);
                    // If it's a fatal error (like invalid username), return it.
                    // Otherwise it might just be the browser closing.
                    if e.to_string().contains("Invalid username") {
                        close_all_tabs(&browser);
                        return Err(e);
                    }
                    return Err(e);
                }
            }

            if retries > MAX_RETRIES {
                close_all_tabs(&browser);
                return Err(anyhow::anyhow!(format!(
                    "Max retries reached. Could not find a handler for the current page: {}",
                    last_url
                )));
            }
        }

        sleep(Duration::from_millis(400));
    }
}
