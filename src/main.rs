mod config;
mod schedule;

use crate::config::Config;
use crate::schedule::{cleanup, manage_battery, SonnenClient};
use chrono::{Local, NaiveTime, TimeDelta};
use log::{info, warn, LevelFilter};
use signal_hook::consts::signal::*;
use signal_hook::iterator::Signals;
use std::fs;
use std::ops::Add;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::mpsc;
use std::time::Duration;
use clap::Parser;
use directories::ProjectDirs;
use systemd_journal_logger::JournalLog;

#[derive(Parser, Debug)]
#[command(author = "shartrec", version = "1.0", about = "Sonnen Evo Manager")]
struct Args {
    /// Optional path to a custom config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Run once and exit (useful for testing)
    #[arg(short, long)]
    dry_run: bool,
}

fn main() {
    let args = Args::parse();

    init_logger();

    info!("Starting Battery Manager...");

    // 1. Resolve Config Path
    let config_path = args.config.unwrap_or_else(|| {
        if let Some(proj_dirs) = ProjectDirs::from("com", "shartrec", "sonnen_manager") {
            // On Linux, this is ~/.config/sonnen_manager/config.toml
            // But for a system service, you might override this via CLI to /etc/sonnen_manager/config.toml
            proj_dirs.config_dir().join("config.toml")
        } else {
            PathBuf::from("config.toml") // Fallback to current dir
        }
    });

    info!("Using config: {:?}", config_path);

    let config_content = fs::read_to_string(&config_path).expect("Could not read config file");
    let config: Config = toml::from_str(&config_content).expect("Invalid TOML format");

    let client = SonnenClient {
        ip: config.battery.ip.clone(),
        token: config.battery.token.clone(),
        dry_run: args.dry_run,
    };

    // have a graceful shutdown mechanism using channels to signal the main loop to exit
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut signals = Signals::new(&[SIGINT, SIGTERM]).unwrap();
        if let Some(sig) = signals.forever().next() {
            info!("Received signal: {}. Cleaning up...", sig);
            // We don't have easy access to config/client here unless we wrap in Arc
            // or just let the main thread handle cleanup after the break.
            let _ = tx.send(());
        }
    });

    loop {

        if let Err(e) = manage_battery(&config, &client) {
            warn!("Error managing battery: {}", e);
        }

        // Check the config and the current time to see How long we
        // need to wait until the next action.
        // For example, if it's 10:00, we can sleep for 1 hour until 11:00 when we need to send the PUT.
        let free_start = config
            .schedule
            .start_time
            .parse::<NaiveTime>()
            .expect("Invalid start_time format in config");
        let free_end = config
            .schedule
            .stop_time
            .parse::<NaiveTime>()
            .expect("Invalid stop_time format in config");

        let now = Local::now().time();
        let sleep_duration_sec = if now > free_start && now < free_end {
            info!("Currently in free window. Will check every minute to ensure schedule is set.");
            Duration::from_secs(60)
        } else {
            let next_action_time = if now < free_start {
                free_start
            } else {
                free_start // Next day's free start? Actually, for simplicity, let's just wait a bit if we're past end
            };
            
            // Re-calculate sleep logic for better robustness
            let mut duration_until_next_action = next_action_time.signed_duration_since(now);
            if duration_until_next_action.num_seconds() <= 0 {
                // If we're past today's start, it might be for tomorrow.
                // We'll wait 5 mins
                duration_until_next_action = TimeDelta::minutes(5);
            }

            info!(
                "Next action at {}. Sleeping for {} seconds, until {}",
                next_action_time,
                duration_until_next_action.num_seconds(),
                now.add(duration_until_next_action)
            );
            Duration::from_secs(duration_until_next_action.num_seconds() as u64)
        };
        // 3. The "Interruptible Sleep"
        // If rx receives a message, it returns Ok(()) immediately.
        // If the duration passes first, it returns Err(Timeout).
        if let Ok(_) = rx.recv_timeout(sleep_duration_sec) {
            info!("Graceful shutdown triggered.");
            cleanup(&config, &client);
            break;
        }
    }
}

fn init_logger() {
    // Initialize journal logging
    if systemd_journal_logger::connected_to_journal() {
        JournalLog::new()
            .unwrap()
            .with_syslog_identifier("sonnen-manager".to_string()) // Name in journalctl
            .install()
            .unwrap();
    } else {
        simple_logger::init().unwrap();
    }
    let log_level = match std::env::var("RUST_LOG") {
        Ok(val) => LevelFilter::from_str(&val).unwrap_or(LevelFilter::Info),
        Err(_) => LevelFilter::Info,
    };

    log::set_max_level(log_level);
}