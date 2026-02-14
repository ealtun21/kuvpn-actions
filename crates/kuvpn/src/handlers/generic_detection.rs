/// Generic page detection utilities for handling unexpected scenarios
use headless_chrome::Tab;

/// Detects any visible error message on the page using common patterns
/// This catches errors we haven't specifically coded for
pub fn detect_generic_error(tab: &Tab) -> anyhow::Result<Option<String>> {
    let js = r#"
        (function() {
            // Check common error patterns
            const errorPatterns = [
                // Elements with error-related attributes
                '[role="alert"]',
                '[aria-live="assertive"]',
                '[aria-live="polite"]',
                // Common error class patterns
                '.error-message',
                '.error-text',
                '.alert-error',
                '.alert-danger',
                '.validation-error',
                '.field-validation-error',
                // Microsoft-specific patterns
                '.ext-error',
                '.error.ext-error',
                // Generic divs with 'error' in ID
                'div[id*="error" i][id*="message" i]',
                'div[id*="error" i]:not([style*="display: none"])',
                'span[id*="error" i]:not([style*="display: none"])',
            ];

            for (const selector of errorPatterns) {
                try {
                    const elements = document.querySelectorAll(selector);
                    for (const el of elements) {
                        // Check if element is visible
                        if (el.offsetParent !== null && el.innerText && el.innerText.trim().length > 0) {
                            const text = el.innerText.trim();
                            // Filter out common false positives
                            if (text.length > 5 && text.length < 500) {
                                return text;
                            }
                        }
                    }
                } catch (e) {
                    // Ignore selector errors, continue with next pattern
                }
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
