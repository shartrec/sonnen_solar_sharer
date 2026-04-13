mod config;
mod schedule;

use crate::config::Config;
use crate::schedule::{cleanup, manage_battery};
use chrono::{Local, NaiveTime};
use log::{info, warn, LevelFilter};
use signal_hook::consts::signal::*;
use signal_hook::iterator::Signals;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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
    once: bool,
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

    let config_content = fs::read_to_string("config.toml").expect("Could not read config.toml");
    let config: Config = toml::from_str(&config_content).expect("Invalid TOML format");

    // Set up signal handling for graceful shutdown
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let shutdown_flag_clone = shutdown_flag.clone();

    std::thread::spawn(move || {
        let mut signals =
            Signals::new(&[SIGTERM, SIGINT]).expect("Failed to register signal handlers");

        for sig in signals.forever() {
            match sig {
                SIGTERM | SIGINT => {
                    info!("Received shutdown signal. Cleaning up...");
                    shutdown_flag_clone.store(true, Ordering::SeqCst);
                    break;
                }
                _ => unreachable!(),
            }
        }
    });

    loop {
        // Check if shutdown signal was received
        if shutdown_flag.load(Ordering::SeqCst) {
            cleanup(&config);
            break;
        }

        if let Err(e) = manage_battery(&config) {
            warn!("Error managing battery: {}", e);
        }

        // Check the config and the current time to see How long we
        // need to wait until the next action.
        // For example, if it's 10:00, we can sleep for 1 hour until 11:00 when we need to send the PUT.
        let free_start = config
            .schedule
            .start_time
            .parse::<NaiveTime>()
            .expect("Invalid stop_time format in config");
        let free_end = config
            .schedule
            .stop_time
            .parse::<NaiveTime>()
            .expect("Invalid stop_time format in config");

        let now = Local::now().time();
        if now > free_start && now < free_end {
            info!("Currently in free window. Will check every minute to ensure schedule is set.");
        } else {
            let next_action_time = if now < free_start {
                free_start
            } else {
                free_end
            };
            let duration_until_next_action = next_action_time.signed_duration_since(now);
            info!(
                "Next action at {}. Sleeping for {} seconds.",
                next_action_time,
                duration_until_next_action.num_seconds()
            );
            std::thread::sleep(std::time::Duration::from_secs(
                duration_until_next_action.num_seconds() as u64,
            ));
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