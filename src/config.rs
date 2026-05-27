use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

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
            client_id: "123456789012345678".to_string(), // Domyślny dummy ID
            presence: PresenceConfig {
                state: Some("Piszę aplikację w Rust".to_string()),
                details: Some("Discord Rich Presence".to_string()),
                large_image: Some("rust".to_string()),
                large_text: Some("Rust Language".to_string()),
                small_image: Some("terminal".to_string()),
                small_text: Some("Konsola".to_string()),
                start_timestamp: true,
                buttons: Some(vec![
                    PresenceButton {
                        label: "Odwiedź Rusta".to_string(),
                        url: "https://www.rust-lang.org".to_string(),
                    },
                    PresenceButton {
                        label: "Mój GitHub".to_string(),
                        url: "https://github.com".to_string(),
                    }
                ]),
            },
        }
    }
}

pub fn load_config<P: AsRef<Path>>(path: P) -> Result<AppConfig, Box<dyn std::error::Error>> {
    let path = path.as_ref();
    if !path.exists() {
        let default_config = AppConfig::default();
        let toml_string = toml::to_string_pretty(&default_config)?;
        let banner = r#"# Konfiguracja Discord Rich Presence (DURCP)
# 
# Aby status działał z Twoimi grafikami:
# 1. Wejdź na https://discord.com/developers/applications
# 2. Stwórz nową aplikację (nazwa aplikacji będzie nazwą "gry" w statusie)
# 3. Skopiuj "Application ID" (Client ID) i wklej go poniżej jako client_id.
# 4. Przejdź do zakładki "Rich Presence" -> "Art Assets" i prześlij grafiki.
# 5. Użyj nazw tych grafik jako large_image i small_image.
# 6. Zmiany w tym pliku są automatycznie wczytywane przez działającą aplikację!

"#;
        let mut file_content = banner.to_string();
        file_content.push_str(&toml_string);
        fs::write(path, file_content)?;
    }

    let content = fs::read_to_string(path)?;
    let config: AppConfig = toml::from_str(&content)?;
    Ok(config)
}
