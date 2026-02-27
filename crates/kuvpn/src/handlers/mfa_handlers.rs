use crate::utils::{CancellationToken, CredentialsProvider};
use headless_chrome::Tab;
use std::collections::HashSet;
use std::thread::sleep;
use std::time::Duration;

/// Handles authenticator push approval.
pub fn handle_authenticator_push_approval(
    tab: &Tab,
    provider: &dyn CredentialsProvider,
    cancel_token: Option<&CancellationToken>,
) -> anyhow::Result<bool> {
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
        let number = tab
            .evaluate(
                r#"(function() {
    var el = document.getElementById('idRichContext_DisplaySign');
    return el ? el.innerText.trim() : '';
})()"#,
                false,
            )?
            .value
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();

        provider.on_mfa_push(&number);

        let prev_url = tab.get_url();
        loop {
            if let Some(token) = cancel_token {
                if token.is_cancelled() {
                    provider.on_mfa_complete();
                    return Err(anyhow::anyhow!("Operation cancelled by user"));
                }
            }

            sleep(Duration::from_secs(1));

            let still_showing = tab
                .evaluate(
                    r#"(function() {
    var el = document.getElementById('idRichContext_DisplaySign');
    return !!(el && el.offsetParent !== null);
})()"#,
                    false,
                )?
                .value
                .unwrap()
                .as_bool()
                .unwrap_or(false);

            if !still_showing {
                provider.on_mfa_complete();
                log::info!("[*] Number prompt gone, continuing...");
                break;
            }

            let new_url = tab.get_url();
            if new_url != prev_url {
                provider.on_mfa_complete();
                log::info!("[*] URL changed, continuing...");
                break;
            }
        }

        return Ok(true);
    }

    Ok(false)
}

/// Handles verification code choice page.
pub fn handle_verification_code_choice(tab: &Tab) -> anyhow::Result<bool> {
    let is_proof_choice_page = tab
        .evaluate(
            r#"(function() {
    var title = document.getElementById('idDiv_SAOTCS_Title');
    return !!(title && title.innerText.trim().toLowerCase().includes('verify your identity'));
})()"#,
            false,
        )?
        .value
        .unwrap()
        .as_bool()
        .unwrap();

    if is_proof_choice_page {
        let _clicked = tab.evaluate(
            r#"(function() {
    var els = document.querySelectorAll('div[role="button"], .table[role="button"], button, input[type="button"]');
    for(var i=0; i<els.length; i++) {
        var text = els[i].innerText.toLowerCase();
        if(text.includes('mobile app') || text.includes('authenticator')) {
            els[i].click();
            return true;
        }
    }
    return false;
})()"#,
            false,
        )?;

        sleep(Duration::from_millis(500));
        return Ok(true);
    }

    Ok(false)
}

/// Handles "Use an app instead" link.
pub fn handle_use_app_instead(tab: &Tab) -> anyhow::Result<bool> {
    let is_visible = tab
        .evaluate(
            r#"(function() {
    var el = document.getElementById('idA_PWD_SwitchToRemoteNGC');
    return !!(el && el.offsetParent !== null);
})()"#,
            false,
        )?
        .value
        .unwrap()
        .as_bool()
        .unwrap();

    if is_visible {
        tab.evaluate(
            r#"var el=document.getElementById('idA_PWD_SwitchToRemoteNGC'); if(el){el.click();}"#,
            false,
        )?;
        log::info!("[*] Clicked 'Use an app instead'");
        sleep(Duration::from_millis(400));
        return Ok(true);
    }

    Ok(false)
}

/// Handles authenticator NGC push notifications.
pub fn handle_authenticator_ngc_push(
    tab: &Tab,
    provider: &dyn CredentialsProvider,
    cancel_token: Option<&CancellationToken>,
) -> anyhow::Result<bool> {
    let is_ngc_push = tab.evaluate(
        r#"(function() {
    var header = document.getElementById('loginHeader') &&
        document.getElementById('loginHeader').innerText.toLowerCase().includes('approve sign in');
    var desc = document.getElementById('idDiv_RemoteNGC_PollingDescription') &&
        document.getElementById('idDiv_RemoteNGC_PollingDescription').innerText.toLowerCase().includes('authenticator app');
    return !!(header && desc);
})()"#,
        false,
    )?.value.unwrap().as_bool().unwrap();

    if is_ngc_push {
        let number = tab
            .evaluate(
                r#"(function() {
    var el=document.getElementById('idRemoteNGC_DisplaySign');
    return (el && el.offsetParent !== null) ? el.innerText.trim() : '';
})()"#,
                false,
            )?
            .value
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();

        provider.on_mfa_push(&number);

        let prev_url = tab.get_url();
        loop {
            if let Some(token) = cancel_token {
                if token.is_cancelled() {
                    provider.on_mfa_complete();
                    return Err(anyhow::anyhow!("Operation cancelled by user"));
                }
            }

            sleep(Duration::from_millis(400));

            let still_showing = tab
                .evaluate(
                    r#"(function() {
    var el=document.getElementById('idRemoteNGC_DisplaySign');
    return !!(el && el.offsetParent !== null);
})()"#,
                    false,
                )?
                .value
                .unwrap()
                .as_bool()
                .unwrap_or(false);

            let new_url = tab.get_url();
            if (!still_showing && !number.is_empty()) || new_url != prev_url {
                provider.on_mfa_complete();
                log::info!("[*] Push page finished, moving on...");
                break;
            }
        }

        return Ok(true);
    }

    Ok(false)
}

/// Handles OTP/verification code entry page.
///
/// Covers SMS codes, email codes, and TOTP codes from the Authenticator app where the
/// user must manually type a code rather than approve a push notification.
/// Detects `input[name="otc"]` which Microsoft uses for all one-time code inputs.
pub fn handle_otp_entry(
    tab: &Tab,
    provider: &dyn CredentialsProvider,
) -> anyhow::Result<bool> {
    let is_otp_page = tab
        .evaluate(
            r#"(function() {
    var input = document.querySelector('input[name="otc"]');
    return !!(input && input.offsetParent !== null);
})()"#,
            false,
        )?
        .value
        .unwrap()
        .as_bool()
        .unwrap_or(false);

    if is_otp_page {
        // Build a prompt from the page description so the user knows what kind of code to enter
        let prompt = tab
            .evaluate(
                r#"(function() {
    var desc = document.getElementById('idDiv_SAOTCC_Description')
        || document.getElementById('idDiv_SAOTCS_Description');
    if (desc && desc.innerText.trim().length > 0) return desc.innerText.trim();
    var title = document.getElementById('idDiv_SAOTCC_Title')
        || document.getElementById('idDiv_SAOTCS_Title');
    if (title && title.innerText.trim().length > 0) return title.innerText.trim();
    return 'Enter verification code';
})()"#,
                false,
            )?
            .value
            .and_then(|v| v.as_str().map(|s| format!("{}: ", s)))
            .unwrap_or_else(|| "Enter verification code: ".to_string());

        log::info!("[*] OTP entry page detected, requesting code from user");

        let code = provider.request_text(&prompt);
        let code_escaped = crate::utils::js_escape(&code);

        tab.evaluate(
            &format!(
                r#"(function() {{
    var el = document.querySelector('input[name="otc"]');
    if (el) {{
        el.focus();
        el.value = '{code}';
        el.dispatchEvent(new Event('input', {{ bubbles: true }}));
        el.dispatchEvent(new Event('change', {{ bubbles: true }}));
    }}
}})()"#,
                code = code_escaped
            ),
            false,
        )?;
        sleep(Duration::from_millis(250));

        // Try the dedicated OTP submit button first, then fall back to the generic Next button
        tab.evaluate(
            r#"(function() {
    var btn = document.querySelector('#idSubmit_SAOTCC_Continue')
        || document.querySelector('#idSubmit_SAOTCS_Continue')
        || document.querySelector('#idSIButton9');
    if (btn) { btn.focus(); btn.click(); }
})()"#,
            false,
        )?;

        return Ok(true);
    }

    Ok(false)
}

/// Handles NGC error and switches to password authentication.
pub fn handle_ngc_error_use_password(
    tab: &Tab,
    handled: &mut HashSet<&'static str>,
) -> anyhow::Result<bool> {
    let is_ngc_error = tab
        .evaluate(
            r#"(function() {
    var header = document.getElementById('loginHeader');
    var errorBlock = document.getElementById('idDiv_RemoteNGC_PageDescription');
    var pollingIndicator = document.getElementById('idDiv_RemoteNGC_PollingDescription');
    var pollingActive = pollingIndicator && pollingIndicator.offsetParent !== null;

    // Text-based detection (English)
    var textMatch = (
        (header && header.innerText.toLowerCase().includes("request wasn't sent")) ||
        (errorBlock && errorBlock.innerText.toLowerCase().includes("couldn't send"))
    );

    // Structural fallback: NGC error div is visible but polling is NOT active
    // (i.e. the push failed rather than being in progress)
    var structuralMatch = (
        errorBlock && errorBlock.offsetParent !== null && !pollingActive
    );

    return !!(textMatch || structuralMatch);
})()"#,
            false,
        )?
        .value
        .unwrap()
        .as_bool()
        .unwrap();

    if is_ngc_error {
        // Try multiple fallback buttons in priority order
        let clicked = tab
            .evaluate(
                r#"(function() {
    var selectors = [
        '#idA_PWD_SwitchToPassword',
        '#signInAnotherWay',
        '#idA_PWD_SwitchToCredPicker'
    ];
    for (var i = 0; i < selectors.length; i++) {
        var el = document.querySelector(selectors[i]);
        if (el && el.offsetParent !== null) {
            el.click();
            return selectors[i];
        }
    }
    return '';
})()"#,
                false,
            )?
            .value
            .unwrap()
            .as_str()
            .unwrap_or("")
            .to_string();

        if !clicked.is_empty() {
            log::info!(
                "[*] NGC error page, switching to password via {}",
                clicked
            );
            handled.insert("use_app_instead");
            sleep(Duration::from_millis(400));
            return Ok(true);
        }
    }

    Ok(false)
}
