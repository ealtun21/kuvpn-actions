use headless_chrome::Tab;

/// Checks if an input element is visible on the page.
pub fn is_input_visible(tab: &Tab, selector: &str) -> anyhow::Result<bool> {
    let js = format!(
        "!!(document.querySelector('{}') && document.querySelector('{}').offsetParent !== null)",
        selector, selector
    );
    Ok(tab.evaluate(&js, false)?.value.unwrap().as_bool().unwrap())
}

/// Checks if the invalid username error message is visible.
pub fn is_invalid_username_visible(tab: &Tab) -> anyhow::Result<bool> {
    let js = "!!(document.getElementById('usernameError') && document.getElementById('usernameError').innerText.includes('We couldn\\'t find an account with that username.'))";
    Ok(tab.evaluate(js, false)?.value.unwrap().as_bool().unwrap())
}