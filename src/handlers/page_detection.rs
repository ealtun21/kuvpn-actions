use headless_chrome::Tab;

/// Checks if an input element is visible on the page.
pub fn is_input_visible(tab: &Tab, selector: &str) -> anyhow::Result<bool> {
    let js = format!(
        "!!(document.querySelector('{}') && document.querySelector('{}').offsetParent !== null)",
        selector, selector
    );
    Ok(tab.evaluate(&js, false)?.value.unwrap().as_bool().unwrap())
}

/// Checks if *any* invalid username error is visible (broader Microsoft error set).
pub fn is_invalid_username_visible(tab: &Tab) -> anyhow::Result<bool> {
    let js = r#"
        !!(document.getElementById('usernameError')
            && (
                document.getElementById('usernameError').innerText.includes("We couldn't find an account with that username.")
                || document.getElementById('usernameError').innerText.toLowerCase().includes("enter a valid email address")
                || document.getElementById('usernameError').innerText.toLowerCase().includes("enter a valid phone number")
                || document.getElementById('usernameError').innerText.toLowerCase().includes("enter a valid skype name")
            )
        )
    "#;
    Ok(tab.evaluate(js, false)?.value.unwrap().as_bool().unwrap())
}

/// Checks if the incorrect password error message is visible.
pub fn is_incorrect_password_visible(tab: &Tab) -> anyhow::Result<bool> {
    let js = "!!(document.getElementById('passwordError') && document.getElementById('passwordError').innerText.includes('Your account or password is incorrect.'))";
    Ok(tab.evaluate(js, false)?.value.unwrap().as_bool().unwrap())
}
