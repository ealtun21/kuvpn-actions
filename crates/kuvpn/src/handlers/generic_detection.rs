/// Generic page detection utilities for handling unexpected scenarios
use headless_chrome::Tab;

/// Detects any visible error message on the page using common patterns
/// This catches errors we haven't specifically coded for
pub fn detect_generic_error(tab: &Tab) -> anyhow::Result<Option<String>> {
    let js = r#"
        (function() {
            // --- High-confidence selectors (class/ID already implies an error) ---
            const errorSelectors = [
                '.error-message',
                '.error-text',
                '.alert-error',
                '.alert-danger',
                '.validation-error',
                '.field-validation-error',
                '.ext-error',
                '.error.ext-error',
                'div[id*="error" i][id*="message" i]',
                'div[id*="error" i]:not([style*="display: none"])',
                'span[id*="error" i]:not([style*="display: none"])',
            ];

            // --- Low-confidence selectors (accessibility attrs used for ALL
            //     dynamic content, not just errors).  We require the text to
            //     contain at least one error-indicative keyword. ---
            const ariaSelectors = [
                '[role="alert"]',
                '[aria-live="assertive"]',
                '[aria-live="polite"]',
            ];

            const errorKeywords = [
                'error', 'failed', 'failure', 'invalid', 'incorrect',
                'denied', 'blocked', 'expired', 'unable', 'problem',
                'went wrong', 'try again', 'couldn\'t', 'could not',
                'not recognized', 'too many', 'locked', 'suspended',
            ];

            // MFA / status text that should never be treated as an error
            const mfaSafe = [
                'enter the code', 'enter code', 'enter the number',
                'enter number', 'verification code', 'verify your',
                'we sent', 'we texted', 'we emailed',
                'approve sign', 'approve the sign', 'approve a sign',
                'open your authenticator', 'check your authenticator',
                'open your microsoft', 'check your microsoft',
                'notification was sent', 'notification to',
                'authenticator app', 'sign in request',
                'waiting for', 'checking', 'contacting',
            ];

            function isVisible(el) {
                return el && el.offsetParent !== null
                    && el.innerText && el.innerText.trim().length > 5
                    && el.innerText.trim().length < 500;
            }

            function isMfaText(lower) {
                return mfaSafe.some(function(p) { return lower.includes(p); });
            }

            // High-confidence: any visible text (after MFA filter) is an error
            for (var s = 0; s < errorSelectors.length; s++) {
                try {
                    var els = document.querySelectorAll(errorSelectors[s]);
                    for (var i = 0; i < els.length; i++) {
                        if (isVisible(els[i])) {
                            var text = els[i].innerText.trim();
                            if (!isMfaText(text.toLowerCase())) return text;
                        }
                    }
                } catch (e) {}
            }

            // Low-confidence: must also contain an error keyword
            for (var s = 0; s < ariaSelectors.length; s++) {
                try {
                    var els = document.querySelectorAll(ariaSelectors[s]);
                    for (var i = 0; i < els.length; i++) {
                        if (isVisible(els[i])) {
                            var text = els[i].innerText.trim();
                            var lower = text.toLowerCase();
                            if (isMfaText(lower)) continue;
                            var hasError = errorKeywords.some(function(k) {
                                return lower.includes(k);
                            });
                            if (hasError) return text;
                        }
                    }
                } catch (e) {}
            }

            return null;
        })()
    "#;

    let result = tab.evaluate(js, false)?;
    if let Some(value) = result.value {
        if value.is_string() {
            let text = value.as_str().unwrap().to_string();
            if !text.is_empty() {
                return Ok(Some(text));
            }
        }
    }
    Ok(None)
}

/// Detects if we're stuck on an unexpected page by analyzing page characteristics
pub fn detect_unexpected_page_state(tab: &Tab) -> anyhow::Result<Option<String>> {
    let js = r#"
        (function() {
            // Get page title
            const title = document.title || '';

            // Check for common "blocked" or "error" keywords in title
            const blockKeywords = [
                'error', 'blocked', 'forbidden', 'denied', 'unauthorized',
                'not found', '404', '403', '500', 'unavailable', 'maintenance'
            ];

            const lowerTitle = title.toLowerCase();
            for (const keyword of blockKeywords) {
                if (lowerTitle.includes(keyword)) {
                    return 'Unexpected page state: ' + title;
                }
            }

            // Check for meta tags indicating errors
            const pageId = document.querySelector('meta[name="PageID"]');
            if (pageId) {
                const content = pageId.content || '';
                if (content.toLowerCase().includes('error')) {
                    return 'Error page detected: ' + content;
                }
            }

            // Check if page has minimal content (might indicate error)
            const bodyText = document.body ? document.body.innerText : '';
            if (bodyText.length < 100 && bodyText.toLowerCase().includes('error')) {
                return 'Minimal content error page: ' + bodyText.trim().substring(0, 100);
            }

            return null;
        })()
    "#;

    let result = tab.evaluate(js, false)?;
    if let Some(value) = result.value {
        if value.is_string() {
            let text = value.as_str().unwrap().to_string();
            if !text.is_empty() {
                return Ok(Some(text));
            }
        }
    }
    Ok(None)
}

/// Attempts to find any submit button on the page (fallback for unknown forms)
pub fn find_submit_button(tab: &Tab) -> anyhow::Result<Option<String>> {
    let js = r#"
        (function() {
            // Look for buttons with submit-related text or types
            const buttonSelectors = [
                'button[type="submit"]',
                'input[type="submit"]',
                'button:not([type="button"])',
            ];

            for (const selector of buttonSelectors) {
                const buttons = document.querySelectorAll(selector);
                for (const btn of buttons) {
                    if (btn.offsetParent !== null) {
                        const text = btn.innerText || btn.value || '';
                        const textLower = text.toLowerCase();

                        // Look for common submit button text
                        if (textLower.includes('next') ||
                            textLower.includes('continue') ||
                            textLower.includes('submit') ||
                            textLower.includes('sign in') ||
                            textLower.includes('login')) {
                            return btn.id || btn.className || selector;
                        }
                    }
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

/// Logs current page state for debugging stuck scenarios
pub fn log_page_state(tab: &Tab) -> anyhow::Result<()> {
    let js = r#"
        (function() {
            const info = {
                title: document.title,
                url: window.location.href,
                pageId: document.querySelector('meta[name="PageID"]')?.content || 'unknown',
                visibleInputs: Array.from(document.querySelectorAll('input[type="text"], input[type="email"], input[type="password"]'))
                    .filter(el => el.offsetParent !== null)
                    .map(el => ({
                        type: el.type,
                        name: el.name,
                        id: el.id,
                        placeholder: el.placeholder
                    })),
                visibleButtons: Array.from(document.querySelectorAll('button, input[type="submit"]'))
                    .filter(el => el.offsetParent !== null)
                    .map(el => ({
                        text: el.innerText || el.value || '',
                        id: el.id,
                        type: el.type
                    })).slice(0, 5), // Limit to first 5 buttons
                headers: Array.from(document.querySelectorAll('h1, h2, .heading, [role="heading"]'))
                    .filter(el => el.offsetParent !== null)
                    .map(el => el.innerText.trim())
                    .filter(text => text.length > 0)
                    .slice(0, 3), // First 3 headers
            };
            return JSON.stringify(info, null, 2);
        })()
    "#;

    let result = tab.evaluate(js, false)?;
    if let Some(value) = result.value {
        if let Some(state) = value.as_str() {
            log::debug!("[Page State] {}", state);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_compiles() {
        // Basic compilation test
        assert!(true);
    }
}
