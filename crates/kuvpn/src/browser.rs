use crate::utils::get_user_data_dir;
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
    // Retrieve the user data directory.
    let user_data_dir = get_user_data_dir()?;

    // Build the command-line arguments for launching the browser.
    let user_agent = OsString::from(format!("--user-agent={agent}"));
    let body = OsString::from("--app=data:text/html,");
    let window = OsString::from("--new-window");

    // Configure the browser launch options.
    let mut options = LaunchOptions::default_builder();
    let mut launch_options = options
        .headless(headless)
        .sandbox(false)
        .idle_browser_timeout(Duration::MAX)
        .window_size(Some((800, 800)))
        .args(vec![
            body.as_os_str(),
            window.as_os_str(),
            user_agent.as_os_str(),
        ])
        .user_data_dir(Some(user_data_dir));

    // If KUVPN_CHROME_PATH is set, use it. Otherwise, use default.
    if let Ok(path) = std::env::var("KUVPN_CHROME_PATH") {
        launch_options = launch_options.path(Some(path.into()));
    } else if let Ok(executable_path) = default_executable() {
        launch_options = launch_options.path(Some(executable_path));
    }

    // Build and return the Browser instance.
    Ok(Browser::new(launch_options.build()?)?)
}
