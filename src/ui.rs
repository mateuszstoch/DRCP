use crate::config::AppConfig;
use colored::*;
use crossterm::{
    cursor,
    execute,
    terminal,
};
use std::io::{stdout, Write};
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Reconnecting(u64), // Seconds remaining for next attempt
}

pub fn render_dashboard(
    status: ConnectionStatus,
    config: &AppConfig,
    uptime: Duration,
    error_msg: Option<&str>,
    config_path: &Path,
    interactive: bool,
) {
    let mut stdout = stdout();
    
    // Reset cursor to top-left and clear everything below it to avoid screen flickering
    let _ = execute!(
        stdout,
        cursor::MoveTo(0, 0),
        terminal::Clear(terminal::ClearType::FromCursorDown)
    );

    let status_str = match status {
        ConnectionStatus::Connected => "CONNECTED".green().bold(),
        ConnectionStatus::Disconnected => "DISCONNECTED".red().bold(),
        ConnectionStatus::Reconnecting(secs) => format!("RECONNECTING IN {}s...", secs).yellow().bold(),
    };

    let seconds = uptime.as_secs() % 60;
    let minutes = (uptime.as_secs() / 60) % 60;
    let hours = uptime.as_secs() / 3600;
    let uptime_str = format!("{:02}:{:02}:{:02}", hours, minutes, seconds).cyan();

    print!("========================================================\r\n");
    print!("     {}      \r\n", "DISCORD RICH PRESENCE (DRCP) - ACTIVE".bold().white());
    print!("========================================================\r\n");
    
    print!("  Status:   {}\r\n", status_str);
    print!("  Uptime:   {}\r\n", uptime_str);
    print!("  Config:   {}\r\n", config_path.display().to_string().cyan());
    print!("--------------------------------------------------------\r\n");
    print!("  Client ID: {}\r\n", config.client_id.magenta());
    
    print!("\r\n  {}\r\n", "Presence Details:".yellow().underline());
    if let Some(ref details) = config.presence.details {
        print!("    Details:      {}\r\n", details);
    } else {
        print!("    Details:      {}\r\n", "<none>".dimmed());
    }
    
    if let Some(ref state) = config.presence.state {
        print!("    State:        {}\r\n", state);
    } else {
        print!("    State:        {}\r\n", "<none>".dimmed());
    }

    if let Some(ref large_img) = config.presence.large_image {
        let large_txt = config.presence.large_text.as_deref().unwrap_or("");
        print!("    Large Image:  {} ({})\r\n", large_img.cyan(), large_txt.dimmed());
    }
    
    if let Some(ref small_img) = config.presence.small_image {
        let small_txt = config.presence.small_text.as_deref().unwrap_or("");
        print!("    Small Image:  {} ({})\r\n", small_img.cyan(), small_txt.dimmed());
    }

    if let Some(ref buttons) = config.presence.buttons {
        if !buttons.is_empty() {
            print!("\r\n  {}\r\n", "Buttons:".yellow().underline());
            for btn in buttons {
                print!("    - {} -> {}\r\n", btn.label.bold(), btn.url.dimmed());
            }
        }
    }

    if let Some(err) = error_msg {
        print!("\r\n  {}\r\n", "Last Error / Log:".red().underline());
        print!("    {}\r\n", err.red());
    }

    print!("\r\n========================================================\r\n");
    print!("  {}\r\n", "Instructions:".white().bold());
    print!("  * Edit {} to dynamically update this status.\r\n", "config.toml".cyan());
    if interactive {
        print!("  * Press {} to configure directly from console.\r\n", "E".yellow().bold());
    }
    print!("  * Press {} to terminate the application.\r\n", "Ctrl+C".red().bold());
    print!("========================================================\r\n");
    
    let _ = stdout.flush();
}
