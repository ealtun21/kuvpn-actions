use playwright::api::StorageState;
use playwright::api::{BrowserContext, Page};
use playwright::Error;
use playwright::Playwright;
use rpassword::read_password;
use std::collections::HashSet;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::time::Duration;

use crate::get_user_data_dir;

// ================= Utility Functions =================

fn js_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'")
}

fn prompt_text(msg: &str) -> String {
    print!("{}", msg);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_owned()
}

fn prompt_password(msg: &str) -> String {
    print!("{}", msg);
    io::stdout().flush().unwrap();
    read_password().unwrap()
}

async fn is_input_visible(page: &Page, selector: &str) -> Result<bool, Arc<Error>> {
    let js = format!(
        "(function() {{
            var el = document.querySelector('{}');
            return !!(el && el.offsetParent !== null);
        }})()",
        selector
    );
    page.eval::<bool>(&js).await
}

/// Returns the full path to the `auth_state.json` inside the user data dir.
pub fn get_auth_state_path() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let mut dir = get_user_data_dir().expect("Unable to get path, do you have file perms?");
    dir.push("auth_state.json");
    Ok(dir)
}

// ================= Page Interaction Helpers =================

async fn fill_on_screen_and_click(
    page: &Page,
    input_selector: &str,
    msg: &str,
    button_selector: &str,
    is_password: bool,
) -> Result<(), Arc<Error>> {
    if is_input_visible(page, input_selector).await? {
        let value = if is_password {
            prompt_password(msg)
        } else {
            prompt_text(msg)
        };
        let value_escaped = js_escape(&value);
        page.eval::<()>(&format!(
            r#"
                var el = document.querySelector('{sel}');
                if (el) {{
                    el.focus();
                    el.value = '{val}';
                    var ev = new Event('input', {{ bubbles: true }});
                    el.dispatchEvent(ev);
                    var ev2 = new Event('change', {{ bubbles: true }});
                    el.dispatchEvent(ev2);
                }}
                void 0;"#,
            sel = input_selector,
            val = value_escaped
        ))
        .await?;
        tokio::time::sleep(Duration::from_millis(250)).await;
        page.eval::<()>(&format!(
            "var btn=document.querySelector('{btn}'); if(btn){{btn.focus();btn.click();}} void 0;",
            btn = button_selector
        ))
        .await?;
    }
    Ok(())
}

async fn click_kmsi_if_present(page: &Page) -> Result<bool, Arc<Error>> {
    let visible = page
        .eval::<bool>(
            "(function() {
            var btn = document.querySelector('#idSIButton9');
            return !!(btn && btn.offsetParent !== null && btn.value === 'Yes');
        })()",
        )
        .await?;
    if visible {
        println!("[*] Detected KMSI – pressing Yes...");
        page.eval::<()>(
            "var chk=document.querySelector('#KmsiCheckboxField'); if(chk && !chk.checked){chk.click();} void 0;"
        ).await?;
        page.eval::<()>(
            "var btn=document.querySelector('#idSIButton9'); if(btn){btn.focus();btn.click();} void 0;"
        ).await?;
        tokio::time::sleep(Duration::from_millis(500)).await;
        return Ok(true);
    }
    Ok(false)
}

// ================= Authenticator & MFA Handlers =================

async fn handle_authenticator_push_approval(page: &Page) -> Result<bool, Arc<Error>> {
    let is_push_page = page.eval::<bool>(
        r#"(function() {
            return !!(
                document.getElementById('idDiv_SAOTCAS_Title') &&
                document.getElementById('idDiv_SAOTCAS_Title').innerText.trim().toLowerCase().includes('approve sign in request') &&
                document.getElementById('idRichContext_DisplaySign')
            );
        })()"#
    ).await?;
    if is_push_page {
        let number: String = page
            .eval(
                r#"(function() {
                var el = document.getElementById('idRichContext_DisplaySign');
                return el ? el.innerText.trim() : '';
            })()"#,
            )
            .await?;
        println!(
            "[*] Push Approval: Please enter this number in your Microsoft Authenticator app: {}",
            number
        );

        let prev_url = page.url()?;
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let still_showing = page
                .eval::<bool>(
                    r#"(function() {
                    var el = document.getElementById('idRichContext_DisplaySign');
                    return !!(el && el.offsetParent !== null);
                })()"#,
                )
                .await
                .unwrap_or(false);
            if !still_showing {
                println!("[*] Number prompt gone, continuing...");
                break;
            }
            let new_url = page.url()?;
            if new_url != prev_url {
                println!("[*] URL changed, continuing...");
                break;
            }
        }
        return Ok(true);
    }
    Ok(false)
}

async fn handle_verification_code_choice(page: &Page) -> Result<bool, Arc<Error>> {
    let is_proof_choice_page = page.eval::<bool>(
        r#"(function() {
            var title = document.getElementById('idDiv_SAOTCS_Title');
            return !!(title && title.innerText.trim().toLowerCase().includes('verify your identity'));
        })()"#
    ).await?;
    if is_proof_choice_page {
        let clicked = page.eval::<bool>(
            r#"(function() {
                var els = document.querySelectorAll('div[role="button"], .table[role="button"], button, input[type="button"]');
                for(var i=0; i<els.length; ++i) {
                    var txt = els[i].innerText || els[i].value || '';
                    if(txt.trim().toLowerCase().includes('verification code')) {
                        els[i].click();
                        return true;
                    }
                }
                return false;
            })()"#
        ).await?;
        if clicked {
            println!("[*] Clicked 'Use a verification code'!");
            tokio::time::sleep(Duration::from_millis(350)).await;
            return Ok(true);
        }
    }
    Ok(false)
}

async fn handle_use_app_instead(page: &Page) -> Result<bool, Arc<Error>> {
    let is_visible = page
        .eval::<bool>(
            r#"(function() {
            var el = document.getElementById('idA_PWD_SwitchToRemoteNGC');
            return !!(el && el.offsetParent !== null);
        })()"#,
        )
        .await?;
    if is_visible {
        page.eval::<()>(
            r#"var el=document.getElementById('idA_PWD_SwitchToRemoteNGC'); if(el){el.click();}"#,
        )
        .await?;
        println!("[*] Clicked 'Use an app instead'");
        tokio::time::sleep(Duration::from_millis(400)).await;
        return Ok(true);
    }
    Ok(false)
}

async fn handle_authenticator_ngc_push(page: &Page) -> Result<bool, Arc<Error>> {
    let is_ngc_push = page.eval::<bool>(
        r#"(function() {
            var header =
              document.getElementById('loginHeader') &&
              document.getElementById('loginHeader').innerText.toLowerCase().includes('approve sign in');
            var desc =
              document.getElementById('idDiv_RemoteNGC_PollingDescription') &&
              document.getElementById('idDiv_RemoteNGC_PollingDescription').innerText.toLowerCase().includes('authenticator app');
            return !!(header && desc);
        })()"#
    ).await?;
    if is_ngc_push {
        let number: String = page
            .eval(
                r#"(function() {
                var el=document.getElementById('idRemoteNGC_DisplaySign');
                return (el && el.offsetParent !== null) ? el.innerText.trim() : '';
            })()"#,
            )
            .await?;
        if !number.is_empty() {
            println!(
                "[*] Push Approval: Enter this number in your MS Authenticator app: {}",
                number
            );
        } else {
            println!("[*] Push Approval: Please approve in your MS Authenticator app");
        }

        let prev_url = page.url()?;
        loop {
            tokio::time::sleep(Duration::from_millis(400)).await;
            let still_showing = page
                .eval::<bool>(
                    r#"(function() {
                    var el=document.getElementById('idRemoteNGC_DisplaySign');
                    return !!(el && el.offsetParent !== null);
                })()"#,
                )
                .await
                .unwrap_or(false);
            let new_url = page.url()?;
            if (!still_showing && !number.is_empty()) || new_url != prev_url {
                println!("[*] Push page finished, moving on...");
                break;
            }
        }
        return Ok(true);
    }
    Ok(false)
}

async fn handle_session_conflict(page: &Page) -> Result<bool, Arc<Error>> {
    let is_conflict_page = page
        .eval::<bool>(
            r#"(function() {
            var form = document.querySelector('#DSIDConfirmForm');
            var btn = document.querySelector('#btnContinue');
            return !!(form && btn);
        })()"#,
        )
        .await?;

    if is_conflict_page {
        println!("[*] Detected existing VPN session. Continuing...");
        page.eval::<()>(r#"var btn=document.getElementById('btnContinue'); if(btn){btn.click();}"#)
            .await?;
        tokio::time::sleep(Duration::from_millis(500)).await;
        return Ok(true);
    }
    Ok(false)
}

async fn handle_ngc_error_use_password(
    page: &Page,
    handled: &mut HashSet<&'static str>,
) -> Result<bool, Arc<Error>> {
    let is_ngc_error = page
        .eval::<bool>(
            r#"(function() {
            var header = document.getElementById('loginHeader');
            var errorBlock = document.getElementById('idDiv_RemoteNGC_PageDescription');
            return !!(
                (header && header.innerText.toLowerCase().includes("request wasn't sent")) ||
                (errorBlock && errorBlock.innerText.toLowerCase().includes("couldn't send"))
            );
        })()"#,
        )
        .await?;
    if is_ngc_error {
        let is_visible = page
            .eval::<bool>(
                r#"(function() {
                var el = document.getElementById('idA_PWD_SwitchToPassword');
                return !!(el && el.offsetParent !== null);
            })()"#,
            )
            .await?;
        if is_visible {
            page.eval::<()>(
                r#"var el=document.getElementById('idA_PWD_SwitchToPassword'); if(el){el.click();}"#
            ).await?;
            println!("[*] NGC error page, switching to password");
            handled.insert("use_app_instead");
            tokio::time::sleep(Duration::from_millis(400)).await;
            return Ok(true);
        }
    }
    Ok(false)
}

// ================= DSID Extractor =================

async fn poll_dsid(context: &BrowserContext, url: &String, domain: &String) -> Result<Option<String>, Error> {
    let cookies = context.cookies(&[url.to_string()]).await?;
    if let Some(dsid) = cookies.iter().find(|c| {
        c.name == "DSID"
            && c.domain
                .as_ref()
                .map_or(false, |d| d.contains(domain))
    }) {
        return Ok(Some(dsid.value.clone()));
    }
    Ok(None)
}

// ================= Library Entry Point =================

pub async fn run_login_and_get_dsid(headless: bool, url: &String, domain: &String) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    const MAX_RETRIES: usize = 10;
    let playwright = Playwright::initialize().await?;
    playwright.prepare()?;
    let chromium = playwright.chromium();

    let browser = chromium
        .launcher()
        .headless(headless)
        .args(&["--no-sandbox".to_string(), "--disable-gpu".to_string()])
        .launch()
        .await?;

    let auth_path = get_auth_state_path()?;

    // Try loading the saved session, properly deserialize to StorageState
    let session_data = tokio::fs::read(&auth_path).await.ok();
    let mut context_builder = browser.context_builder().user_agent("Mozilla/5.0");
    if let Some(data) = session_data {
        if let Ok(storage_state) = serde_json::from_slice::<StorageState>(&data) {
            println!("[+] Attempting to reuse saved authentication state...");
            context_builder = context_builder.storage_state(storage_state);
        }
    }

    let context = context_builder.build().await?;
    let page = context.new_page().await?;

    println!("[+] Starting at: {}", url);
    page.goto_builder(url).goto().await?;
    
    // A helper function to try all handlers and return if any was successful.
    async fn try_handle_page(page: &Page, handled: &mut HashSet<&'static str>) -> Result<bool, Arc<Error>> {
        if !handled.contains("session_conflict") && handle_session_conflict(page).await? {
            handled.insert("session_conflict");
            return Ok(true);
        }
        if !handled.contains("username") && is_input_visible(page, "input[name=\"loginfmt\"]").await? {
            fill_on_screen_and_click(page, "input[name=\"loginfmt\"]", "Username (email): ", "#idSIButton9", false).await?;
            handled.insert("username");
            return Ok(true);
        }
        if !handled.contains("ngc_error_use_password") && handle_ngc_error_use_password(page, handled).await? {
            handled.insert("ngc_error_use_password");
            return Ok(true);
        }
        if !handled.contains("use_app_instead") && handle_use_app_instead(page).await? {
            handled.insert("use_app_instead");
            return Ok(true);
        }
        if !handled.contains("ngc_push") && handle_authenticator_ngc_push(page).await? {
            handled.insert("ngc_push");
            return Ok(true);
        }
        if !handled.contains("password") && is_input_visible(page, "input[name=\"passwd\"]").await? {
            fill_on_screen_and_click(page, "input[name=\"passwd\"]", "Password: ", "#idSIButton9", true).await?;
            handled.insert("password");
            return Ok(true);
        }
        if !handled.contains("kmsi") && click_kmsi_if_present(page).await? {
            handled.insert("kmsi");
            return Ok(true);
        }
        if !handled.contains("push") && handle_authenticator_push_approval(page).await? {
            handled.insert("push");
            return Ok(true);
        }
        if !handled.contains("verification_code") && handle_verification_code_choice(page).await? {
            handled.insert("verification_code");
            return Ok(true);
        }
        Ok(false)
    }

    let mut handled: HashSet<&'static str> = HashSet::new();
    let mut last_url = page.url()?;
    let mut retries = 0;

    loop {
        // Try to obtain DSID from existing (possibly loaded) cookies
        if let Some(dsid) = poll_dsid(&context, url, domain).await? {
            let state = context.storage_state().await?;
            let state_json = serde_json::to_vec(&state)?;
            tokio::fs::write(&auth_path, &state_json).await?;
            browser.close().await?;
            println!("[✓] Found valid dsid, closing browser.");
            return Ok(dsid);
        }

        let current_url = page.url()?;
        if current_url != last_url {
            println!("[*] Page navigated to: {}", current_url);
            last_url = current_url;
            retries = 0; // Reset retries on navigation
        }

        // Try to handle one of the login steps.
        let handled_something = try_handle_page(&page, &mut handled).await?;

        if handled_something {
            retries = 0; // Reset retries if something was handled.
        } else {
            retries += 1;
        }

        if retries > MAX_RETRIES {
            browser.close().await?;
            return Err(Box::new(io::Error::new(io::ErrorKind::Other, format!("Max retries reached. Could not find a handler for the current page: {}", last_url))));
        }

        tokio::time::sleep(Duration::from_millis(400)).await;
    }
}