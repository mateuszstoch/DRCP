# DRCP - Discord Rich Presence CLI

A lightweight console utility written in Rust to set and manage custom Discord Rich Presence (DRP) statuses. It supports Windows, macOS, and Linux.

It features file-watching on `config.toml`, meaning you can edit your status in any text editor and the CLI will instantly update your Discord presence without needing a restart.

---

## Features

- **Live updates**: Automatically watches `config.toml` and pushes status changes immediately.
- **Terminal UI**: Displays active presence configurations, connection status, and uptime.
- **Automatic reconnection**: If Discord is closed or restarted, the client handles socket reconnection attempts every 5 seconds.
- **Graceful shutdown**: Clears your Discord presence and restores terminal cursor visibility on exit (`Ctrl+C`).

---

## Installation

### For Users (Precompiled Binaries)

#### macOS & Linux
Run the following script in your terminal to fetch and install the latest binary:
```bash
curl -sSfL https://raw.githubusercontent.com/mateuszstoch/DRCP/main/scripts/install.sh | sh
```
*(Alternatively, download the target binary for your architecture from the GitHub Releases page).*

#### Windows
Run the following command in PowerShell to fetch and install the latest binary:
```powershell
irm https://raw.githubusercontent.com/mateuszstoch/DRCP/main/scripts/install.ps1 | iex
```
*(Alternatively, download the target zip archive from the GitHub Releases page, extract it, and run the executable).*

---

### For Developers (Build from Source)

If you have the Rust toolchain installed:
```bash
git clone https://github.com/mateuszstoch/DRCP.git
cd DRCP
cargo run --release
```

On first startup, the app creates a default `config.toml` in your working directory.

---

## Configuration

All presence options are defined in `config.toml`. 

By default:
- On **macOS and Linux**, the config file is located at `~/.config/drcp/config.toml`.
- On **Windows**, the config file is located at `%APPDATA%\drcp\config.toml`.
- If a `config.toml` is found in the current working directory, DRCP will use it instead (useful for local development).

The structure of the `config.toml` file is as follows:

```toml
client_id = "123456789012345678" # Your Discord application Client ID

[presence]
state = "Writing Rust code"
details = "Discord Rich Presence"
large_image = "rust"            # Key name of the asset uploaded to Discord portal
large_text = "Rust Language"
small_image = "terminal"        # Key name of the small overlay asset
small_text = "Console"
start_timestamp = true          # Shows time elapsed since DRCP started

[[presence.buttons]]
label = "Rust Home"
url = "https://www.rust-lang.org"

[[presence.buttons]]
label = "GitHub"
url = "https://github.com"
```

---

## Discord Developer Portal Setup

To customize the application name, logos, and hover-texts:

1. Go to the [Discord Developer Portal](https://discord.com/developers/applications).
2. Create a new Application. The application's name will appear as your main playing status (*"Playing [Name]"*).
3. Copy the **Application ID** (Client ID) and paste it into the `client_id` field in your `config.toml`.
4. To add images:
   - Select your application in the portal.
   - Go to **Rich Presence** -> **Art Assets**.
   - Add images under **Rich Presence Assets** and name them in lowercase (e.g. `rust`, `logo`).
   - Use these keys in your `config.toml` for `large_image` and `small_image`.
5. *Note: Discord does not let you click your own Rich Presence buttons, but they will be active and clickable for other users.*

---

## Usage

1. Run the `drcp` executable.
2. The CLI will open the active dashboard showing your status.
3. **Interactive Console Editing**: Press `E` in your terminal to open the interactive configuration editor. You can update your Client ID, details, images, and buttons directly from the console. DRCP will save the updates to your config file and apply them instantly!
4. **Manual Editing**: Alternatively, edit your config file directly (DRCP displays the path to the active `config.toml` in the UI). The CLI will pick up changes on file save.
5. Exit by pressing `Ctrl+C`.

---

## License

MIT
