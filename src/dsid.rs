use headless_chrome::{Browser, LaunchOptionsBuilder, Tab};
use rpassword::read_password;
use std::{
    collections::HashSet,
    io::{self, Write},
    path::PathBuf,
    thread::sleep,
    time::Duration,
};

use crate::get_user_data_dir;

// ================= Utility Functions =================

fn js_escape(s: &str) -> String {
    s.replace("\\", "\\\\").replace("'", "\\'")
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

fn get_auth_state_path() -> PathBuf {
    let mut dir = get_user_data_dir().expect("Cannot get user data dir");
    dir.push("auth_state.json");
    dir
}

// ================= Page Interaction Helpers =================

fn is_input_visible(tab: &Tab, selector: &str) -> anyhow::Result<bool> {
    let js = format!(
        "!!(document.querySelector('{}') && document.querySelector('{}').offsetParent !== null)",
        selector, selector
    );
    Ok(tab.evaluate(&js, false)?.value.unwrap().as_bool().unwrap())
}

fn fill_on_screen_and_click(
    tab: &Tab,
    input_selector: &str,
    msg: &str,
    button_selector: &str,
    is_password: bool,
) -> anyhow::Result<()> {
    if is_input_visible(tab, input_selector)? {
        let value = if is_password {
            prompt_password(msg)
        } else {
            prompt_text(msg)
        };
        let value_escaped = js_escape(&value);
        let js = format!(
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
            "#,
            sel = input_selector, val = value_escaped
        );
        tab.evaluate(&js, false)?;
        sleep(Duration::from_millis(250));
        let js_btn = format!(
            "var btn=document.querySelector('{btn}'); if(btn){{btn.focus();btn.click();}}",
            btn = button_selector
        );
        tab.evaluate(&js_btn, false)?;
    }
    Ok(())
}

fn click_kmsi_if_present(tab: &Tab) -> anyhow::Result<bool> {
    let js = r#"
        (function() {
            var btn = document.querySelector('#idSIButton9');
            return !!(btn && btn.offsetParent !== null && btn.value === 'Yes');
        })()
    "#;
    let visible = tab.evaluate(js, false)?.value.unwrap().as_bool().unwrap();
    if visible {
        println!("[*] Detected KMSI – pressing Yes...");
        tab.evaluate("var chk=document.querySelector('#KmsiCheckboxField'); if(chk && !chk.checked){chk.click();}", false)?;
        tab.evaluate("var btn=document.querySelector('#idSIButton9'); if(btn){btn.focus();btn.click();}", false)?;
        sleep(Duration::from_millis(500));
        return Ok(true);
    }
    Ok(false)
}

// ================= Authenticator & MFA Handlers =================

fn handle_authenticator_push_approval(tab: &Tab) -> anyhow::Result<bool> {
    let is_push_page = tab.evaluate(
        r#"(function() {
            return !!(
                document.getElementById('idDiv_SAOTCAS_Title') &&
                document.getElementById('idDiv_SAOTCAS_Title').innerText.trim().toLowerCase().includes('approve sign in request') &&
                document.getElementById('idRichContext_DisplaySign')
            );
        })()"#,
        false,
    )?.value.unwrap().as_bool().unwrap();
    if is_push_page {
        let number = tab.evaluate(
            r#"(function() {
                var el = document.getElementById('idRichContext_DisplaySign');
                return el ? el.innerText.trim() : '';
            })()"#,
            false,
        )?.value.unwrap().as_str().unwrap().to_string();

        println!(
            "[*] Push Approval: Please enter this number in your Microsoft Authenticator app: {}",
            number
        );
        let mut prev_url = tab.get_url();
        loop {
            sleep(Duration::from_secs(1));
            let still_showing = tab.evaluate(
                r#"(function() {
                    var el = document.getElementById('idRichContext_DisplaySign');
                    return !!(el && el.offsetParent !== null);
                })()"#,
                false,
            )?.value.unwrap().as_bool().unwrap_or(false);

            if !still_showing {
                println!("[*] Number prompt gone, continuing...");
                break;
            }
            let new_url = tab.get_url();
            if new_url != prev_url {
                println!("[*] URL changed, continuing...");
                break;
            }
        }
        return Ok(true);
    }
    Ok(false)
}

fn handle_verification_code_choice(tab: &Tab) -> anyhow::Result<bool> {
    let is_proof_choice_page = tab.evaluate(
        r#"(function() {
            var title = document.getElementById('idDiv_SAOTCS_Title');
            return !!(title && title.innerText.trim().toLowerCase().includes('verify your identity'));
        })()"#,
        false,
    )?.value.unwrap().as_bool().unwrap();

    if is_proof_choice_page {
        let clicked = tab.evaluate(
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
            })()"#,
            false,
        )?.value.unwrap().as_bool().unwrap();

        if clicked {
            println!("[*] Clicked 'Use a verification code'!");
            sleep(Duration::from_millis(350));
            return Ok(true);
        }
    }
    Ok(false)
}

fn handle_use_app_instead(tab: &Tab) -> anyhow::Result<bool> {
    let is_visible = tab.evaluate(
        r#"(function() {
            var el = document.getElementById('idA_PWD_SwitchToRemoteNGC');
            return !!(el && el.offsetParent !== null);
        })()"#,
        false,
    )?.value.unwrap().as_bool().unwrap();
    if is_visible {
        tab.evaluate(
            r#"var el=document.getElementById('idA_PWD_SwitchToRemoteNGC'); if(el){el.click();}"#,
            false,
        )?;
        println!("[*] Clicked 'Use an app instead'");
        sleep(Duration::from_millis(400));
        return Ok(true);
    }
    Ok(false)
}

fn handle_authenticator_ngc_push(tab: &Tab) -> anyhow::Result<bool> {
    let is_ngc_push = tab.evaluate(
        r#"(function() {
            var header =
              document.getElementById('loginHeader') &&
              document.getElementById('loginHeader').innerText.toLowerCase().includes('approve sign in');
            var desc =
              document.getElementById('idDiv_RemoteNGC_PollingDescription') &&
              document.getElementById('idDiv_RemoteNGC_PollingDescription').innerText.toLowerCase().includes('authenticator app');
            return !!(header && desc);
        })()"#,
        false,
    )?.value.unwrap().as_bool().unwrap();

    if is_ngc_push {
        let number = tab.evaluate(
            r#"(function() {
                var el=document.getElementById('idRemoteNGC_DisplaySign');
                return (el && el.offsetParent !== null) ? el.innerText.trim() : '';
            })()"#,
            false,
        )?.value.unwrap().as_str().unwrap().to_string();

        if !number.is_empty() {
            println!(
                "[*] Push Approval: Enter this number in your MS Authenticator app: {}",
                number
            );
        } else {
            println!("[*] Push Approval: Please approve in your MS Authenticator app");
        }
        let mut prev_url = tab.get_url();
        loop {
            sleep(Duration::from_millis(400));
            let still_showing = tab.evaluate(
                r#"(function() {
                    var el=document.getElementById('idRemoteNGC_DisplaySign');
                    return !!(el && el.offsetParent !== null);
                })()"#,
                false,
            )?.value.unwrap().as_bool().unwrap_or(false);

            let new_url = tab.get_url();
            if (!still_showing && !number.is_empty()) || new_url != prev_url {
                println!("[*] Push page finished, moving on...");
                break;
            }
        }
        return Ok(true);
    }
    Ok(false)
}

fn handle_session_conflict(tab: &Tab) -> anyhow::Result<bool> {
    let is_conflict_page = tab.evaluate(
        r#"(function() {
            var form = document.querySelector('#DSIDConfirmForm');
            var btn = document.querySelector('#btnContinue');
            return !!(form && btn);
        })()"#,
        false,
    )?.value.unwrap().as_bool().unwrap();

    if is_conflict_page {
        println!("[*] Detected existing VPN session. Continuing...");
        tab.evaluate(
            r#"var btn=document.getElementById('btnContinue'); if(btn){btn.click();}"#,
            false,
        )?;
        sleep(Duration::from_millis(500));
        return Ok(true);
    }
    Ok(false)
}

fn handle_ngc_error_use_password(tab: &Tab, handled: &mut HashSet<&'static str>) -> anyhow::Result<bool> {
    let is_ngc_error = tab.evaluate(
        r#"(function() {
            var header = document.getElementById('loginHeader');
            var errorBlock = document.getElementById('idDiv_RemoteNGC_PageDescription');
            return !!(
                (header && header.innerText.toLowerCase().includes("request wasn't sent")) ||
                (errorBlock && errorBlock.innerText.toLowerCase().includes("couldn't send"))
            );
        })()"#,
        false,
    )?.value.unwrap().as_bool().unwrap();

    if is_ngc_error {
        let is_visible = tab.evaluate(
            r#"(function() {
                var el = document.getElementById('idA_PWD_SwitchToPassword');
                return !!(el && el.offsetParent !== null);
            })()"#,
            false,
        )?.value.unwrap().as_bool().unwrap();

        if is_visible {
            tab.evaluate(
                r#"var el=document.getElementById('idA_PWD_SwitchToPassword'); if(el){el.click();}"#,
                false,
            )?;
            println!("[*] NGC error page, switching to password");
            handled.insert("use_app_instead");
            sleep(Duration::from_millis(400));
            return Ok(true);
        }
    }
    Ok(false)
}

// ================= DSID Extractor =================

fn poll_dsid(tab: &Tab, domain: &str) -> anyhow::Result<Option<String>> {
    let cookies = tab.get_cookies()?;
    if let Some(cookie) = cookies.iter().find(|c| c.name == "DSID" && c.domain.contains(domain)) {
        return Ok(Some(cookie.value.clone()));
    }
    Ok(None)
}

// ================= Helper Function: Try All Handlers =================

fn try_handle_page(tab: &Tab, handled: &mut HashSet<&'static str>) -> anyhow::Result<bool> {
    if !handled.contains("session_conflict") && handle_session_conflict(tab)? {
        handled.insert("session_conflict");
        return Ok(true);
    }
    if !handled.contains("username") && is_input_visible(tab, "input[name=\"loginfmt\"]")? {
        fill_on_screen_and_click(tab, "input[name=\"loginfmt\"]", "Username (email): ", "#idSIButton9", false)?;
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
        fill_on_screen_and_click(tab, "input[name=\"passwd\"]", "Password: ", "#idSIButton9", true)?;
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

// ================= Library Entry Point =================

pub fn run_login_and_get_dsid(headless: bool, url: &str, domain: &str) -> anyhow::Result<String> {
    const MAX_RETRIES: usize = 10;
    let browser = Browser::new(
        LaunchOptionsBuilder::default()
            .headless(headless)
            .build()
            .unwrap(),
    )?;
    let tab = browser.new_tab()?;
    tab.navigate_to(url)?;

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
        sleep(Duration::from_millis(400));
    }
}
