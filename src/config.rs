use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PresenceButton {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PresenceConfig {
    pub state: Option<String>,
    pub details: Option<String>,
    pub large_image: Option<String>,
    pub large_text: Option<String>,
    pub small_image: Option<String>,
    pub small_text: Option<String>,
    #[serde(default = "default_start_timestamp")]
    pub start_timestamp: bool,
    pub buttons: Option<Vec<PresenceButton>>,
}

fn default_start_timestamp() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub client_id: String,
    pub presence: PresenceConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            client_id: "123456789012345678".to_string(), // Default dummy ID
            presence: PresenceConfig {
                state: Some("Writing Rust code".to_string()),
                details: Some("Discord Rich Presence".to_string()),
                large_image: Some("rust".to_string()),
                large_text: Some("Rust Language".to_string()),
                small_image: Some("terminal".to_string()),
                small_text: Some("Console".to_string()),
                start_timestamp: true,
                buttons: Some(vec![
                    PresenceButton {
                        label: "Visit Rust".to_string(),
                        url: "https://www.rust-lang.org".to_string(),
                    },
                    PresenceButton {
                        label: "GitHub".to_string(),
                        url: "https://github.com".to_string(),
                    }
                ]),
            },
        }
    }
}

pub fn get_config_path() -> PathBuf {
    // If config.toml exists in the current working directory, use it (local dev fallback)
    let local_path = PathBuf::from("config.toml");
    if local_path.exists() {
        return local_path;
    }

    // Otherwise, resolve standard config directory:
    // macOS/Linux: ~/.config/drcp/config.toml
    // Windows: %APPDATA%/drcp/config.toml
    let mut path = if cfg!(windows) {
        if let Ok(appdata) = std::env::var("APPDATA") {
            PathBuf::from(appdata)
        } else {
            PathBuf::from(".")
        }
    } else {
        if let Ok(home) = std::env::var("HOME") {
            let mut p = PathBuf::from(home);
            p.push(".config");
            p
        } else {
            PathBuf::from(".")
        }
    };

    path.push("drcp");
    let _ = fs::create_dir_all(&path);
    path.push("config.toml");
    path
}

pub fn save_config(config: &AppConfig, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let toml_string = toml::to_string_pretty(config)?;
    let banner = r#"# Discord Rich Presence (DRCP) Configuration
# 
# To make the presence work with your custom assets:
# 1. Go to https://discord.com/developers/applications
# 2. Create a new application (the application's name will appear as "Playing [Name]")
# 3. Copy the "Application ID" (Client ID) and paste it below as client_id.
# 4. Go to "Rich Presence" -> "Art Assets" in the side menu and upload your images.
# 5. Use the exact asset keys as large_image and small_image.
# 6. Changes to this file are automatically reloaded by the running application!

"#;
    let mut file_content = banner.to_string();
    file_content.push_str(&toml_string);
    fs::write(path, file_content)?;
    Ok(())
}

pub fn load_config<P: AsRef<Path>>(path: P) -> Result<AppConfig, Box<dyn std::error::Error>> {
    let path = path.as_ref();
    if !path.exists() {
        let default_config = AppConfig::default();
        save_config(&default_config, path)?;
    }

    let content = fs::read_to_string(path)?;
    let config: AppConfig = toml::from_str(&content)?;
    Ok(config)
}
