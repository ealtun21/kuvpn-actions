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
                || document.getElementById('usernameError').innerText.includes("This username may be incorrect")
                || document.getElementById('usernameError').innerText.includes("Make sure you typed it correctly")
            )
        )
    "#;
    Ok(tab.evaluate(js, false)?.value.unwrap().as_bool().unwrap())
}

/// Checks if a username warning (soft error) is visible on the page.
/// This detects the Azure AD warning: "This username may be incorrect. Make sure you typed it correctly."
pub fn is_username_warning_visible(tab: &Tab) -> anyhow::Result<Option<String>> {
    let js = r#"
        (function() {
            const usernameError = document.getElementById('usernameError');
            if (usernameError) {
                const text = usernameError.innerText;
                if (text.includes("This username may be incorrect") ||
                    text.includes("Make sure you typed it correctly")) {
                    return text;
                }
            }

            // Also check for error alerts or banners that might contain this warning
            const alerts = document.querySelectorAll('[role="alert"], .alert-error, .error-message');
            for (let alert of alerts) {
                const text = alert.innerText;
                if (text.includes("This username may be incorrect") ||
                    text.includes("Make sure you typed it correctly")) {
                    return text;
                }
            }

            return null;
        })()
    "#;

    let result = tab.evaluate(js, false)?;
    if let Some(value) = result.value {
        if value.is_string() {
            Ok(Some(value.as_str().unwrap().to_string()))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

/// Checks if the incorrect password error message is visible.
pub fn is_incorrect_password_visible(tab: &Tab) -> anyhow::Result<bool> {
    let js = "!!(document.getElementById('passwordError') && document.getElementById('passwordError').innerText.includes('Your account or password is incorrect.'))";
    Ok(tab.evaluate(js, false)?.value.unwrap().as_bool().unwrap())
}
