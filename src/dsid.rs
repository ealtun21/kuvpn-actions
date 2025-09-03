use crate::browser::create_browser;
use crate::handlers::page_detection::is_invalid_username_visible;
use crate::handlers::{auth_handlers::*, mfa_handlers::*, page_detection::is_input_visible};
use headless_chrome::Tab;
use std::collections::HashSet;
use std::thread::sleep;
use std::time::Duration;

/// Extracts DSID cookie from the browser.
fn poll_dsid(tab: &Tab, domain: &str) -> anyhow::Result<Option<String>> {
    let cookies = tab.get_cookies()?;
    if let Some(cookie) = cookies
        .iter()
        .find(|c| c.name == "DSID" && c.domain.contains(domain))
    {
        return Ok(Some(cookie.value.clone()));
    }

    Ok(None)
}

/// Attempts to handle the current page state.
fn try_handle_page(tab: &Tab, handled: &mut HashSet<&'static str>) -> anyhow::Result<bool> {
    if !handled.contains("session_conflict") && handle_session_conflict(tab)? {
        handled.insert("session_conflict");
        return Ok(true);
    }
    
    if !handled.contains("invalid_username") && is_invalid_username_visible(tab)? {
        handled.insert("invalid_username");
        return Err(anyhow::anyhow!("Invalid username or account not found"));
    }

    if !handled.contains("username") && is_input_visible(tab, "input[name=\"loginfmt\"]")? {
        fill_on_screen_and_click(
            tab,
            "input[name=\"loginfmt\"]",
            "Username (email): ",
            "#idSIButton9",
            false,
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

    if !handled.contains("ngc_push") && handle_authenticator_ngc_push(tab)? {
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
        )?;
        handled.insert("password");
        return Ok(true);
    }

    if !handled.contains("kmsi") && click_kmsi_if_present(tab)? {
        handled.insert("kmsi");
        return Ok(true);
    }

    if !handled.contains("push") && handle_authenticator_push_approval(tab)? {
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
) -> anyhow::Result<String> {
    const MAX_RETRIES: usize = 10;

    let browser = match create_browser(user_agent, headless) {
        Ok(b) => b,
        Err(e) => return Err(anyhow::anyhow!(format!("Failed to create browser: {e}"))),
    };

    // Use the initial tab provided by the browser.
    #[allow(deprecated)]
    let tab = browser.wait_for_initial_tab()?;

    // Navigate to the target URL and wait for the page to load.
    tab.navigate_to(url)?;
    tab.wait_until_navigated()?;

    let mut handled: HashSet<&'static str> = HashSet::new();
    let mut last_url = tab.get_url();
    let mut retries = 0;

    loop {
        if let Some(dsid) = poll_dsid(&tab, domain)? {
            println!("[✓] Found valid DSID, quitting.");
            tab.close(true)?;
            return Ok(dsid);
        }

        let current_url = tab.get_url();
        if current_url != last_url {
            println!("[*] Page navigated to: {}", current_url);
            last_url = current_url;
            retries = 0;
        }
        
        if !no_auto_login {
            let handled_something = try_handle_page(&tab, &mut handled)?;
            if handled_something {
                retries = 0;
            } else {
                retries += 1;
            }

            if retries > MAX_RETRIES {
                tab.close(true)?;
                return Err(anyhow::anyhow!(format!(
                    "Max retries reached. Could not find a handler for the current page: {}",
                    last_url
                )));
            }
        }
        
        sleep(Duration::from_millis(400));
    }
}