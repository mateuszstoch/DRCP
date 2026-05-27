mod config;
mod ui;

use config::load_config;
use ui::{render_dashboard, ConnectionStatus};

use discord_rich_presence::{
    activity::{Activity, Assets, Button, Timestamps},
    DiscordIpc, DiscordIpcClient,
};
use notify::{RecursiveMode, Watcher};
use std::{
    path::Path,
    sync::mpsc::channel,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const CONFIG_PATH: &str = "config.toml";
const RECONNECT_DELAY_SECS: u64 = 5;

#[derive(Debug)]
enum AppEvent {
    ReloadConfig,
    Tick,
    Exit,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initial configuration setup
    let mut config = match load_config(CONFIG_PATH) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Błąd podczas wczytywania konfiguracji: {}", e);
            eprintln!("Upewnij się, że plik {} jest poprawny.", CONFIG_PATH);
            std::process::exit(1);
        }
    };

    // Prepare channel for events
    let (tx, rx) = channel();

    // 2. Setup Ctrl+C handler
    let tx_ctrlc = tx.clone();
    ctrlc::set_handler(move || {
        let _ = tx_ctrlc.send(AppEvent::Exit);
    })?;

    // 3. Setup File Watcher
    let tx_watcher = tx.clone();
    // Keep watcher alive by assigning it to a variable
    let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
        if let Ok(event) = res {
            // Only trigger reload on write or modify events to prevent spamming
            if event.kind.is_modify() || event.kind.is_create() {
                let _ = tx_watcher.send(AppEvent::ReloadConfig);
            }
        }
    })?;
    let _ = watcher.watch(Path::new(CONFIG_PATH), RecursiveMode::NonRecursive);

    // 4. Setup Tick Thread
    let tx_tick = tx.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(1));
            if tx_tick.send(AppEvent::Tick).is_err() {
                break;
            }
        }
    });

    // Send initial tick to trigger immediate connection attempt
    let _ = tx.send(AppEvent::Tick);

    // Hide terminal cursor for better UI experience
    let mut stdout = std::io::stdout();
    let _ = crossterm::execute!(stdout, crossterm::cursor::Hide);

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
    
    // We update status every 15s to keep it alive/detect drops
    let mut last_activity_update = Instant::now();
    let mut force_update = true;

    // Main loop
    loop {
        // Render dashboard
        render_dashboard(
            status,
            &config,
            start_time.elapsed(),
            error_msg.as_deref(),
        );

        // Wait for next event
        let event = match rx.recv() {
            Ok(evt) => evt,
            Err(_) => break,
        };

        match event {
            AppEvent::ReloadConfig => {
                // Reload configuration
                match load_config(CONFIG_PATH) {
                    Ok(new_cfg) => {
                        // Check if client_id changed
                        let client_id_changed = new_cfg.client_id != config.client_id;
                        config = new_cfg;
                        error_msg = None;
                        
                        if client_id_changed {
                            // Close old client and reset to force new creation
                            if let Some(mut old_client) = client.take() {
                                let _ = old_client.close();
                            }
                            status = ConnectionStatus::Disconnected;
                            reconnect_timer = 0;
                        }
                        
                        force_update = true;
                    }
                    Err(e) => {
                        error_msg = Some(format!("Błąd przeładowania pliku config: {}", e));
                    }
                }
            }
            AppEvent::Tick => {
                let is_connected = status == ConnectionStatus::Connected;
                
                if !is_connected {
                    if reconnect_timer > 0 {
                        reconnect_timer -= 1;
                        status = ConnectionStatus::Reconnecting(reconnect_timer);
                    } else {
                        // Attempt connection
                        status = ConnectionStatus::Reconnecting(0);
                        render_dashboard(status, &config, start_time.elapsed(), error_msg.as_deref());
                        
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
                                error_msg = Some(format!("Błąd połączenia z IPC Discorda: {}", e));
                                status = ConnectionStatus::Reconnecting(RECONNECT_DELAY_SECS);
                                reconnect_timer = RECONNECT_DELAY_SECS;
                            }
                        }
                    }
                } else if is_connected {
                    // Check if we need to update activity (every 15s or when forced)
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
                                    error_msg = Some(format!("Błąd wysyłania aktywności: {}", e));
                                    status = ConnectionStatus::Disconnected;
                                    reconnect_timer = RECONNECT_DELAY_SECS;
                                    client = None;
                                }
                            }
                        }
                    }
                }
            }
            AppEvent::Exit => {
                // Restore terminal cursor
                let _ = crossterm::execute!(std::io::stdout(), crossterm::cursor::Show);
                
                // Clear Discord status before exiting
                if let Some(mut cl) = client.take() {
                    let _ = cl.clear_activity();
                    let _ = cl.close();
                }
                
                // Clear screen to leave terminal clean
                let _ = crossterm::execute!(
                    std::io::stdout(),
                    crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
                    crossterm::cursor::MoveTo(0, 0)
                );
                
                println!("DURCP zakończył działanie. Miłego dnia!");
                break;
            }
        }
    }

    Ok(())
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
