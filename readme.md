# Sonnen Solar Sharer

A Rust-based service that automatically manages charging schedules for Sonnen battery systems during off-peak electricity hours, maximizing the use of solar energy and reducing energy costs.

## Features

- **Automatic Time-of-Use (ToU) Scheduling**: Automatically sets and clears charging schedules based on configured free electricity windows
- **Operating Mode Management**: Switches the battery between Manual, Auto, and Time-of-Use modes
- **Dry Run Mode**: Test your configuration without sending commands to the battery
- **Graceful Signal Handling**: Properly cleans up and exits when receiving SIGTERM or SIGINT signals
- **Systemd Integration**: Full support for running as a systemd service with journal logging
- **Configuration File Support**: TOML-based configuration for easy setup
- **Comprehensive Logging**: Detailed debug and info logging to systemd journal or stderr

## Requirements

- Rust 1.70 or later
- Access to a Sonnen battery system (Evo or compatible) via network
- Battery API token for authentication

## Installation

### Build from Source

```bash
git clone <repository-url>
cd sonnen_solar_sharer
cargo build --release
```

The compiled binary will be available at `target/release/sonnen_solar_sharer`.

### Systemd Service Setup

1. Copy the binary to `/usr/local/bin/`:
   ```bash
   sudo cp target/release/sonnen_solar_sharer /usr/local/bin/
   ```

2. Create a systemd service file at `/etc/systemd/system/sonnen-solar-sharer.service`:
   ```ini
   [Unit]
   Description=Sonnen Solar Sharer Battery Manager
   After=network-online.target
   Wants=network-online.target

   [Service]
   Type=simple
   ExecStart=/usr/local/bin/sonnen_solar_sharer
   Restart=on-failure
   RestartSec=10
   StandardOutput=journal
   StandardError=journal

   [Install]
   WantedBy=multi-user.target
   ```

3. Enable and start the service:
   ```bash
   sudo systemctl daemon-reload
   sudo systemctl enable sonnen-solar-sharer
   sudo systemctl start sonnen-solar-sharer
   ```

## Configuration

Create a `config.toml` file in the working directory (or where you run the binary from):

```toml
[battery]
ip = "192.168.1.100"              # Battery system IP address
token = "your-api-token-here"     # Battery API authentication token
max_charge_watt = 5000            # Maximum charging rate in watts (5000W for Evo)

[schedule]
start_time = "11:00"              # Start of free electricity window (24-hour format)
stop_time = "14:00"               # End of free electricity window (24-hour format)
```

### Configuration Parameters

| Parameter | Description | Example |
|-----------|-------------|---------|
| `battery.ip` | IP address of the Sonnen battery system | `192.168.1.100` |
| `battery.token` | API authentication token from battery settings | `abc123def456` |
| `battery.max_charge_watt` | Maximum charging rate in watts | `5000` |
| `schedule.start_time` | Start time of free electricity window (HH:MM) | `11:00` |
| `schedule.stop_time` | End time of free electricity window (HH:MM) | `14:00` |

## Usage

### Run with Default Configuration

```bash
./sonnen_solar_sharer
```

### Dry Run Mode (No Changes to Battery)

```bash
./sonnen_solar_sharer --dry-run
```

In dry run mode, the application will:
- Read the configuration file
- Simulate API calls without sending them to the battery
- Log what would have been sent as formatted JSON

### View Logs

```bash
# Follow systemd journal logs in real-time
journalctl -u sonnen-solar-sharer -f

# View all logs for the service
journalctl -u sonnen-solar-sharer

# Show debug-level logs
journalctl -u sonnen-solar-sharer -g DEBUG
```

## How It Works

1. **Starts Up**: Reads configuration and initializes connection to the battery
2. **Main Loop**: Periodically checks the current time against the configured free electricity window
3. **During Free Window**: 
   - If schedule is not set, configures the battery to charge at maximum rate
   - Sets operating mode to Time-of-Use (ToU)
   - Monitors to ensure schedule remains active
4. **Outside Free Window**:
   - Clears the charging schedule
   - Sets operating mode back to Auto
5. **On Shutdown**: Clears the schedule and sets operating mode to Auto for clean shutdown

## Architecture

### Modules

- **main.rs**: Application entry point, signal handling, and main loop
- **schedule.rs**: Battery client implementation and scheduling logic
- **config.rs**: Configuration file parsing and data structures

### Design Patterns

- **Trait-Based Abstraction**: `BatteryClient` trait allows for easy testing with mock implementations
- **Functional Error Handling**: Uses Rust's `Result` type with `.map()` and `.map_err()` for clean error handling
- **Signal-Safe Cleanup**: Gracefully handles shutdown signals without losing state

## Development

### Running Tests

```bash
cargo test
```

### Building Documentation

```bash
cargo doc --open
```

## Troubleshooting

### Connection Issues
- Verify the battery IP address is reachable: `ping 192.168.1.100`
- Confirm the API token is correct by testing with curl:
  ```bash
  curl -H "Auth-Token: your-token" http://192.168.1.100/api/v2/configurations/EM_ToU_Schedule
  ```

### Schedule Not Applying
- Check that the current time is within the configured free window
- Review logs for API errors: `journalctl -u sonnen-solar-sharer -f`
- Verify the battery is not in a mode that prevents schedule changes

### High CPU Usage
- Ensure the main loop sleep duration is appropriate for your use case
- Check for excessive logging at DEBUG level

## License

This project is licensed under the GNU General Public License v2.0 - see the LICENSE file for details.

**sonnen_solar_sharer** is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation; either version 2 of the License, or (at your option) any later version.

**sonnen_solar_sharer** is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.

You should have received a copy of the GNU General Public License along with **sonnen_solar_sharer**; if not, write to the Free Software Foundation, Inc., 59 Temple Place, Suite 330, Boston, MA 02111-1307 USA

## Support

For issues, questions, or contributions, please [add support information here].

## Dependencies

- **ureq**: HTTP client for battery API communication
- **serde**: Serialization framework for configuration and API data
- **chrono**: Date/time handling
- **toml**: TOML configuration file parsing
- **log**: Logging facade
- **systemd-journal-logger**: Systemd journal integration
- **signal-hook**: Unix signal handling
- **clap**: Command-line argument parsing
- **directories**: Platform-specific directory handling

## Version

Current version: 0.1.0

## Changelog

### 0.1.0
- Initial release
- Basic ToU scheduling
- Signal handling and graceful shutdown
- Dry run mode for testing
- Systemd integration

