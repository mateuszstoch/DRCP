mod config;
mod ui;

use config::{get_config_path, load_config};
use ui::{render_dashboard, ConnectionStatus};

use discord_rich_presence::{
    activity::{Activity, Assets, Button, Timestamps},
    DiscordIpc, DiscordIpcClient,
};
use notify::{RecursiveMode, Watcher};
use std::{
    io::{IsTerminal, Write},
    path::Path,
    sync::mpsc::channel,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const RECONNECT_DELAY_SECS: u64 = 5;

#[derive(Debug)]
enum AppEvent {
    ReloadConfig,
}

struct RawModeGuard {
    active: bool,
}

impl RawModeGuard {
    fn new() -> Self {
        let is_terminal = std::io::stdin().is_terminal();
        let active = is_terminal && crossterm::terminal::enable_raw_mode().is_ok();
        if active {
            let _ = crossterm::execute!(std::io::stdout(), crossterm::cursor::Hide);
        }
        Self { active }
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        if self.active {
            let _ = crossterm::terminal::disable_raw_mode();
            let _ = crossterm::execute!(std::io::stdout(), crossterm::cursor::Show);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initial configuration setup
    let config_path = get_config_path();
    let mut config = match load_config(&config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Error loading configuration: {}", e);
            eprintln!("Please check if your config file is valid: {}", config_path.display());
            std::process::exit(1);
        }
    };

    // Prepare channel for file watcher events
    let (tx, rx) = channel();

    // 2. Setup File Watcher (watching parent directory is more robust for safe saves)
    let tx_watcher = tx.clone();
    let watch_file = config_path.clone();
    let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
        if let Ok(event) = res {
            if event.kind.is_modify() || event.kind.is_create() {
                if event.paths.iter().any(|p| p == &watch_file) {
                    let _ = tx_watcher.send(AppEvent::ReloadConfig);
                }
            }
        }
    })?;
    if let Some(parent) = config_path.parent() {
        let _ = watcher.watch(parent, RecursiveMode::NonRecursive);
    }

    // Enable raw mode and hide cursor (if supported by the TTY)
    let guard = RawModeGuard::new();
    let interactive = guard.active;

    // State variables
    let start_time = Instant::now();
    let start_timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    let mut client: Option<DiscordIpcClient> = None;
    let mut status = ConnectionStatus::Disconnected;
    let mut error_msg: Option<String> = None;
    let mut reconnect_timer = 0;
    
    let mut last_tick = Instant::now() - Duration::from_secs(1); // Force immediate tick logic on startup
    let mut last_activity_update = Instant::now();
    let mut force_update = true;

    // Main loop
    loop {
        // 1. Process file watcher config reload events (non-blocking)
        while let Ok(evt) = rx.try_recv() {
            match evt {
                AppEvent::ReloadConfig => {
                    match load_config(&config_path) {
                        Ok(new_cfg) => {
                            let client_id_changed = new_cfg.client_id != config.client_id;
                            config = new_cfg;
                            error_msg = None;
                            
                            if client_id_changed {
                                if let Some(mut old_client) = client.take() {
                                    let _ = old_client.close();
                                }
                                status = ConnectionStatus::Disconnected;
                                reconnect_timer = 0;
                            }
                            
                            force_update = true;
                        }
                        Err(e) => {
                            error_msg = Some(format!("Error reloading config: {}", e));
                        }
                    }
                }
            }
        }

        // 2. Poll for terminal key input (if running in an interactive TTY)
        if interactive {
            if crossterm::event::poll(Duration::from_millis(50))? {
                if let crossterm::event::Event::Key(key_event) = crossterm::event::read()? {
                    // Filter key presses to avoid release/repeat duplication
                    if key_event.kind == crossterm::event::KeyEventKind::Press {
                        use crossterm::event::{KeyCode, KeyModifiers};

                        // Ctrl+C to exit
                        if key_event.code == KeyCode::Char('c') && key_event.modifiers.contains(KeyModifiers::CONTROL) {
                            break;
                        }

                        // E to enter edit menu
                        if key_event.code == KeyCode::Char('e') || key_event.code == KeyCode::Char('E') {
                            // Temporarily disable raw mode and show cursor
                            let _ = crossterm::terminal::disable_raw_mode();
                            let _ = crossterm::execute!(std::io::stdout(), crossterm::cursor::Show);

                            handle_edit_menu(&mut config, &config_path);

                            // Re-enable raw mode and hide cursor
                            let _ = crossterm::terminal::enable_raw_mode();
                            let _ = crossterm::execute!(std::io::stdout(), crossterm::cursor::Hide);

                            force_update = true;
                        }
                    }
                }
            }
        } else {
            // Non-interactive fallback: just sleep briefly to avoid CPU pinning
            std::thread::sleep(Duration::from_millis(50));
        }

        // 3. Process tick logic (every 1 second)
        if last_tick.elapsed() >= Duration::from_secs(1) {
            last_tick = Instant::now();
            
            let is_connected = status == ConnectionStatus::Connected;
            
            if !is_connected {
                if reconnect_timer > 0 {
                    reconnect_timer -= 1;
                    status = ConnectionStatus::Reconnecting(reconnect_timer);
                } else {
                    status = ConnectionStatus::Reconnecting(0);
                    render_dashboard(status, &config, start_time.elapsed(), error_msg.as_deref(), &config_path, interactive);
                    
                    let mut new_client = DiscordIpcClient::new(&config.client_id);
                    match new_client.connect() {
                        Ok(()) => {
                            client = Some(new_client);
                            status = ConnectionStatus::Connected;
                            error_msg = None;
                            force_update = true;
                            last_activity_update = Instant::now();
                        }
                        Err(e) => {
                            error_msg = Some(format!("Error connecting to Discord IPC: {}", e));
                            status = ConnectionStatus::Reconnecting(RECONNECT_DELAY_SECS);
                            reconnect_timer = RECONNECT_DELAY_SECS;
                        }
                    }
                }
            } else if is_connected {
                if force_update || last_activity_update.elapsed() >= Duration::from_secs(15) {
                    if let Some(ref mut cl) = client {
                        let activity = build_activity(&config.presence, start_timestamp_ms);
                        match cl.set_activity(activity) {
                            Ok(()) => {
                                error_msg = None;
                                force_update = false;
                                last_activity_update = Instant::now();
                            }
                            Err(e) => {
                                error_msg = Some(format!("Error updating activity: {}", e));
                                status = ConnectionStatus::Disconnected;
                                reconnect_timer = RECONNECT_DELAY_SECS;
                                client = None;
                            }
                        }
                    }
                }
            }

            // Render dashboard on every tick
            render_dashboard(
                status,
                &config,
                start_time.elapsed(),
                error_msg.as_deref(),
                &config_path,
                interactive,
            );
        }
    }

    // Clean exit
    if let Some(mut cl) = client.take() {
        let _ = cl.clear_activity();
        let _ = cl.close();
    }
    
    // Clear screen to leave terminal clean (only if raw mode was active)
    if interactive {
        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
            crossterm::cursor::MoveTo(0, 0)
        );
    }
    println!("DRCP stopped. Goodbye!");

    Ok(())
}

fn handle_edit_menu(config: &mut config::AppConfig, config_path: &Path) {
    loop {
        // Clear screen using ANSI escapes
        print!("{}[2J{}[1;1H", 27 as char, 27 as char);
        println!("========================================================");
        println!("              EDIT PRESENCE CONFIGURATION               ");
        println!("========================================================");
        println!("  1. Client ID          : {}", config.client_id);
        println!("  2. Details            : {}", config.presence.details.as_deref().unwrap_or(""));
        println!("  3. State              : {}", config.presence.state.as_deref().unwrap_or(""));
        println!("  4. Large Image Key    : {}", config.presence.large_image.as_deref().unwrap_or(""));
        println!("  5. Large Image Text   : {}", config.presence.large_text.as_deref().unwrap_or(""));
        println!("  6. Small Image Key    : {}", config.presence.small_image.as_deref().unwrap_or(""));
        println!("  7. Small Image Text   : {}", config.presence.small_text.as_deref().unwrap_or(""));
        println!("  8. Configure Buttons  ");
        println!("  9. Save & Back to Dashboard");
        println!("========================================================");
        print!("Select an option (1-9): ");
        let _ = std::io::stdout().flush();

        let mut choice = String::new();
        if std::io::stdin().read_line(&mut choice).is_err() {
            break;
        }

        let choice = choice.trim();
        if choice == "9" {
            if let Err(e) = config::save_config(config, config_path) {
                println!("Error saving config: {}", e);
                std::thread::sleep(Duration::from_secs(2));
            }
            break;
        }

        match choice {
            "1" => {
                print!("Enter Client ID: ");
                let _ = std::io::stdout().flush();
                let mut val = String::new();
                let _ = std::io::stdin().read_line(&mut val);
                config.client_id = val.trim().to_string();
            }
            "2" => {
                print!("Enter Details: ");
                let _ = std::io::stdout().flush();
                let mut val = String::new();
                let _ = std::io::stdin().read_line(&mut val);
                config.presence.details = Some(val.trim().to_string()).filter(|s| !s.is_empty());
            }
            "3" => {
                print!("Enter State: ");
                let _ = std::io::stdout().flush();
                let mut val = String::new();
                let _ = std::io::stdin().read_line(&mut val);
                config.presence.state = Some(val.trim().to_string()).filter(|s| !s.is_empty());
            }
            "4" => {
                print!("Enter Large Image Key: ");
                let _ = std::io::stdout().flush();
                let mut val = String::new();
                let _ = std::io::stdin().read_line(&mut val);
                config.presence.large_image = Some(val.trim().to_string()).filter(|s| !s.is_empty());
            }
            "5" => {
                print!("Enter Large Image Hover Text: ");
                let _ = std::io::stdout().flush();
                let mut val = String::new();
                let _ = std::io::stdin().read_line(&mut val);
                config.presence.large_text = Some(val.trim().to_string()).filter(|s| !s.is_empty());
            }
            "6" => {
                print!("Enter Small Image Key: ");
                let _ = std::io::stdout().flush();
                let mut val = String::new();
                let _ = std::io::stdin().read_line(&mut val);
                config.presence.small_image = Some(val.trim().to_string()).filter(|s| !s.is_empty());
            }
            "7" => {
                print!("Enter Small Image Hover Text: ");
                let _ = std::io::stdout().flush();
                let mut val = String::new();
                let _ = std::io::stdin().read_line(&mut val);
                config.presence.small_text = Some(val.trim().to_string()).filter(|s| !s.is_empty());
            }
            "8" => {
                println!("\n--- Current Buttons ---");
                if let Some(ref btns) = config.presence.buttons {
                    for (i, btn) in btns.iter().enumerate() {
                        println!("{}. {} -> {}", i + 1, btn.label, btn.url);
                    }
                } else {
                    println!("No buttons configured.");
                }
                print!("\nWould you like to (c)lear all buttons, or (a)dd a new button? (c/a/back): ");
                let _ = std::io::stdout().flush();
                let mut btn_choice = String::new();
                let _ = std::io::stdin().read_line(&mut btn_choice);
                let btn_choice = btn_choice.trim().to_lowercase();
                
                if btn_choice == "c" {
                    config.presence.buttons = None;
                } else if btn_choice == "a" {
                    let mut buttons = config.presence.buttons.clone().unwrap_or_default();
                    if buttons.len() >= 2 {
                        println!("Discord only supports up to 2 buttons! Clear existing buttons first.");
                        std::thread::sleep(Duration::from_secs(2));
                        continue;
                    }
                    print!("Enter button label: ");
                    let _ = std::io::stdout().flush();
                    let mut label = String::new();
                    let _ = std::io::stdin().read_line(&mut label);
                    
                    print!("Enter button URL: ");
                    let _ = std::io::stdout().flush();
                    let mut url = String::new();
                    let _ = std::io::stdin().read_line(&mut url);
                    
                    buttons.push(config::PresenceButton {
                        label: label.trim().to_string(),
                        url: url.trim().to_string(),
                    });
                    config.presence.buttons = Some(buttons);
                }
            }
            _ => {}
        }
    }
}

fn build_activity<'a>(presence: &'a config::PresenceConfig, start_time_ms: i64) -> Activity<'a> {
    let mut activity = Activity::new();
    
    if let Some(ref state) = presence.state {
        activity = activity.state(state);
    }
    
    if let Some(ref details) = presence.details {
        activity = activity.details(details);
    }
    
    if presence.start_timestamp {
        activity = activity.timestamps(Timestamps::new().start(start_time_ms));
    }
    
    // Assets
    if presence.large_image.is_some() || presence.small_image.is_some() {
        let mut assets = Assets::new();
        if let Some(ref large_img) = presence.large_image {
            assets = assets.large_image(large_img);
            if let Some(ref large_txt) = presence.large_text {
                assets = assets.large_text(large_txt);
            }
        }
        if let Some(ref small_img) = presence.small_image {
            assets = assets.small_image(small_img);
            if let Some(ref small_txt) = presence.small_text {
                assets = assets.small_text(small_txt);
            }
        }
        activity = activity.assets(assets);
    }
    
    // Buttons
    if let Some(ref buttons) = presence.buttons {
        let mut rpc_buttons = Vec::new();
        for btn in buttons.iter().take(2) { // Discord allows max 2 buttons
            rpc_buttons.push(Button::new(&btn.label, &btn.url));
        }
        activity = activity.buttons(rpc_buttons);
    }
    
    activity
}
