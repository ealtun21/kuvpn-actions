use headless_chrome::browser::default_executable;
use headless_chrome::{Browser, LaunchOptions};
use std::error::Error;
use std::ffi::OsString;
use std::time::Duration;

/// Creates a browser instance configured with a blank page, a custom user agent,
/// and a dedicated user data directory.
///
/// The browser is launched with the following settings:
/// - **Headless or non-headless mode** based on the `headless` parameter.
/// - **A blank app window** containing a minimal HTML page.
/// - **Custom user agent** as specified by the `agent` parameter.
/// - **Custom user data directory** for isolated session data.
/// - **Custom window size** of 800x800 pixels.
/// - **Sandbox disabled** and an infinite idle timeout.
///
/// # Arguments
///
/// * `agent` - A string slice representing the desired user agent.
/// * `headless` - Whether to run the browser in headless mode.
///
/// # Returns
///
/// A `Result` containing the configured `Browser` instance on success, or an error on failure.
pub fn create_browser(agent: &str, headless: bool) -> Result<Browser, Box<dyn Error>> {
    let user_agent = OsString::from(format!("--user-agent={agent}"));
    let window = OsString::from("--new-window");
    let no_first_run = OsString::from("--no-first-run");
    let no_default_browser_check = OsString::from("--no-default-browser-check");
    let disable_session_crashed_bubble = OsString::from("--disable-session-crashed-bubble");

    let mut attempts = 0;
    loop {
        let user_data_dir = crate::utils::get_user_data_dir()?;

        let mut options = LaunchOptions::default_builder();
        let mut launch_options = options
            .headless(headless)
            .sandbox(false)
            .idle_browser_timeout(Duration::MAX)
            .window_size(Some((800, 800)))
            .enable_gpu(false)
            .args(vec![
                window.as_os_str(),
                user_agent.as_os_str(),
                no_first_run.as_os_str(),
                no_default_browser_check.as_os_str(),
                disable_session_crashed_bubble.as_os_str(),
            ])
            .user_data_dir(Some(user_data_dir));

        if let Ok(path) = std::env::var("KUVPN_CHROME_PATH") {
            launch_options = launch_options.path(Some(path.into()));
        } else if let Ok(executable_path) = default_executable() {
            launch_options = launch_options.path(Some(executable_path));
        }

        match Browser::new(launch_options.build()?) {
            Ok(browser) => return Ok(browser),
            Err(e) => {
                attempts += 1;
                if attempts >= 2 {
                    return Err(format!("Browser failed even after wipe: {}", e).into());
                }
                log::warn!("[!] Browser connection failed. Wiping profile and retrying...");
                crate::utils::wipe_user_data_dir()?;
            }
        }
    }
}
