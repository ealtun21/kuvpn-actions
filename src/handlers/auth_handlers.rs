use crate::handlers::page_detection::is_input_visible;
use crate::utils::{js_escape, prompt_password, prompt_text};
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
        println!("[*] Detected KMSI – pressing Yes...");
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
