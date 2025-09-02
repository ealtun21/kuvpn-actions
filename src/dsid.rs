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

fn is_input_visible(tab: &Tab, selector: &str) -> anyhow::Result<bool> {
    let js = format!(
        "!!(document.querySelector('{}') && document.querySelector('{}').offsetParent !== null)",
        selector, selector
    );
    let v: bool = tab.evaluate(&js, false)?.value.unwrap().as_bool().unwrap();
    Ok(v)
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
                el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                el.dispatchEvent(new Event('change', {{ bubbles: true }}));
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

// Example for main loop
pub fn run_login_and_get_dsid(headless: bool, url: &str, domain: &str) -> anyhow::Result<String> {
    let browser = Browser::new(
        LaunchOptionsBuilder::default()
            .headless(headless)
            .build()
            .unwrap(),
    )?;
    let tab = browser.wait_for_initial_tab()?;
    tab.navigate_to(url)?;

    let mut handled: HashSet<&'static str> = HashSet::new();

    loop {
        // Implement poll_dsid analogously by reading cookies via tab.get_cookies()
        let cookies = tab.get_cookies()?;
        if let Some(cookie) = cookies.iter().find(|c| c.name == "DSID" && c.domain.contains(domain)) {
            return Ok(cookie.value.clone());
        }
        // Example interaction for username:
        if !handled.contains("username") && is_input_visible(&tab, "input[name=\"loginfmt\"]")? {
            fill_on_screen_and_click(&tab, "input[name=\"loginfmt\"]", "Username (email): ", "#idSIButton9", false)?;
            handled.insert("username");
            continue;
        }

        sleep(Duration::from_millis(400));
    }
}
