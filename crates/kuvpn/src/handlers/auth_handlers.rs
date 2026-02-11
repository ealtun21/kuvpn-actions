use crate::handlers::page_detection::is_input_visible;
use crate::utils::{js_escape, CredentialsProvider};
use headless_chrome::Tab;
use std::thread::sleep;
use std::time::Duration;

/// Fills an input field on screen and clicks a button.
pub fn fill_on_screen_and_click(
    tab: &Tab,
    input_selector: &str,
    msg: &str,
    button_selector: &str,
    is_password: bool,
    value: Option<&String>,
    provider: &dyn CredentialsProvider,
) -> anyhow::Result<()> {
    if is_input_visible(tab, input_selector)? {
        let value = if let Some(v) = value {
            v.to_owned()
        } else if is_password {
            provider.request_password(msg)
        } else {
            provider.request_text(msg)
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
            sel = input_selector,
            val = value_escaped
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

/// Clicks the "Keep me signed in" button if present.
pub fn click_kmsi_if_present(tab: &Tab) -> anyhow::Result<bool> {
    let js = r#"
(function() {
    var btn = document.querySelector('#idSIButton9');
    return !!(btn && btn.offsetParent !== null && btn.value === 'Yes');
})()
"#;

    let visible = tab.evaluate(js, false)?.value.unwrap().as_bool().unwrap();
    if visible {
        log::info!("[*] Detected KMSI – pressing Yes...");
        tab.evaluate(
            "var chk=document.querySelector('#KmsiCheckboxField'); if(chk && !chk.checked){chk.click();}",
            false
        )?;
        tab.evaluate(
            "var btn=document.querySelector('#idSIButton9'); if(btn){btn.focus();btn.click();}",
            false,
        )?;
        sleep(Duration::from_millis(500));
        return Ok(true);
    }

    Ok(false)
}

/// Handles session conflict page.
pub fn handle_session_conflict(tab: &Tab) -> anyhow::Result<bool> {
    let is_conflict_page = tab
        .evaluate(
            r#"(function() {
    var form = document.querySelector('#DSIDConfirmForm');
    var btn = document.querySelector('#btnContinue');
    return !!(form && btn);
})()"#,
            false,
        )?
        .value
        .unwrap()
        .as_bool()
        .unwrap();

    if is_conflict_page {
        log::info!("[*] Detected existing VPN session. Continuing...");
        tab.evaluate(
            r#"var btn=document.getElementById('btnContinue'); if(btn){btn.click();}"#,
            false,
        )?;
        sleep(Duration::from_millis(500));
        return Ok(true);
    }

    Ok(false)
}

/// Detects the "Request denied" Authenticator page and presses Next.
pub fn handle_remote_ngc_denied_next(tab: &headless_chrome::Tab) -> anyhow::Result<bool> {
    let is_denied_page = tab
        .evaluate(
            r#"(function() {
            var header = document.getElementById('loginHeader');
            var desc = document.getElementById('idDiv_RemoteNGC_PageDescription');
            var form = document.getElementById('i0281');
            var btn = document.getElementById('idSIButton9');
            // Check for specific header and error text
            return !!(
                form && btn &&
                header &&
                header.innerText.trim().toLowerCase() === "request denied" &&
                desc && desc.innerText.toLowerCase().includes("but you denied it")
            );
        })()"#,
            false,
        )?
        .value
        .unwrap()
        .as_bool()
        .unwrap_or(false);

    if is_denied_page {
        log::info!("[*] Authenticator denied page detected. Pressing Next...");
        tab.evaluate(
            "var btn=document.getElementById('idSIButton9'); if(btn){btn.focus();btn.click();}",
            false,
        )?;
        std::thread::sleep(std::time::Duration::from_millis(500));
        return Ok(true);
    }
    Ok(false)
}

/// Detects the "Pick an account" account picker and selects the first account.
pub fn handle_pick_account(tab: &headless_chrome::Tab) -> anyhow::Result<bool> {
    let is_picker = tab.evaluate(
        r#"(function() {
            var header = document.getElementById('loginHeader');
            var form = document.getElementById('i0281');
            var tiles = document.querySelectorAll('#tilesHolder .tile[role="listitem"], #tilesHolder .tile-container .table[role="button"]');
            // Check for specific header and presence of account tiles
            return !!(
                form &&
                header &&
                header.innerText.trim().toLowerCase() === "pick an account" &&
                tiles.length > 0
            );
        })()"#,
        false,
    )?.value.unwrap().as_bool().unwrap_or(false);

    if is_picker {
        log::info!("[*] 'Pick an account' page detected. Selecting the first account...");
        // Click the first available tile button
        tab.evaluate(
            r#"(function() {
                var btn = document.querySelector('#tilesHolder .tile-container .table[role="button"], #tilesHolder .tile[role="listitem"] .table[role="button"]');
                if (btn) { btn.focus(); btn.click(); }
            })()"#,
            false,
        )?;
        std::thread::sleep(std::time::Duration::from_millis(500));
        return Ok(true);
    }
    Ok(false)
}
