use crate::theme::ThemeConfig;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GuiSettings {
    pub url: String,
    pub domain: String,
    pub email: String,
    pub escalation_tool: String,
    pub log_level_val: f32,
    pub openconnect_path: String,
    pub login_mode_val: f32,
    pub close_to_tray: bool,
    pub use_client_decorations: bool,
    #[serde(default = "default_auto_hide")]
    pub auto_hide_after_prompt: bool,
    #[serde(default)]
    pub advanced_mode: bool,
    #[serde(default)]
    pub theme: ThemeConfig,
    /// Route all traffic through the VPN (full tunnel mode).
    #[serde(default)]
    pub full_tunnel: bool,
    /// Path to a custom vpnc-script passed via --script to openconnect (advanced).
    #[serde(default)]
    pub vpnc_script: String,
}

fn default_auto_hide() -> bool {
    true
}

impl Default for GuiSettings {
    fn default() -> Self {
        Self {
            url: "https://vpn.ku.edu.tr".to_string(),
            domain: "vpn.ku.edu.tr".to_string(),
            email: String::new(),
            escalation_tool: "sudo".to_string(),
            log_level_val: 3.0, // Default: Info level
            openconnect_path: "openconnect".to_string(),
            login_mode_val: 0.0,
            close_to_tray: true,
            use_client_decorations: true,
            auto_hide_after_prompt: true,
            advanced_mode: false,
            theme: ThemeConfig::default(),
            full_tunnel: true,
            vpnc_script: String::new(),
        }
    }
}

impl GuiSettings {
    pub fn save(&self) -> anyhow::Result<()> {
        let dir = kuvpn::utils::get_user_data_dir().map_err(|e| anyhow::anyhow!("{}", e))?;
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }
        let path = dir.join("gui_settings.json");
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load() -> Self {
        if let Ok(dir) = kuvpn::utils::get_user_data_dir() {
            let path = dir.join("gui_settings.json");
            if path.exists() {
                if let Ok(content) = fs::read_to_string(path) {
                    if let Ok(settings) = serde_json::from_str(&content) {
                        return settings;
                    }
                }
            }
        }
        Self::default()
    }
}
