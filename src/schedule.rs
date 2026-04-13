use crate::config::Config;
use chrono::{Local, NaiveTime};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Serialize, Deserialize, Debug)]
pub struct TouEntry {
    pub start: String,          // e.g., "11:00"
    pub stop: String,           // e.g., "14:00"
    pub threshold_p_max: i32,   // 5000 for max charge on Evo
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TouSchedule {
    #[serde(rename = "EM_ToU_Schedule")]
    pub entries: Vec<TouEntry>,
}

pub fn manage_battery(config: &Config) -> Result<(), Box<dyn Error>> {
    let now = Local::now().time();
    let free_start = config.schedule.start_time.parse::<NaiveTime>()
        .expect("Invalid start_time format in config");
    let free_end = config.schedule.stop_time.parse::<NaiveTime>()
        .expect("Invalid start_time format in config");

    let url = format!("http://{}/api/v2/configurations/EM_ToU_Schedule", config.battery.ip);

    // 1. Get current config from Battery
    let mut response = ureq::get(&url)
        .header("Auth-Token", &config.battery.token)
        .call()?;
    let current_schedule: TouSchedule = response.body_mut().read_json()?;

    // 2. Determine what SHOULD be happening
    let is_free_window = now >= free_start && now < free_end;

    if is_free_window && current_schedule.entries.is_empty() {
        // ACTION: It's free time but battery isn't told to charge. Send the PUT.
        let _ = set_tou_schedule(config, config.battery.max_charge_watt); // 5kW charge
    } else if !is_free_window && !current_schedule.entries.is_empty() {
        // ACTION: Free time is over. Clear the schedule.
        let _ = clear_tou_schedule(config); // Sends []
    }
    Ok(())
}

pub fn set_tou_schedule(config: &Config, threshold_p_max: i32) -> Result<(), Box<dyn Error>> {
    let start = config.schedule.start_time.clone();
    let stop = config.schedule.stop_time.clone();
    let new_schedule = TouSchedule {
        entries: vec![TouEntry {
            start: start.clone(),
            stop: stop.clone(),
            threshold_p_max: 5000,
        }],
    };

    send_tou_shedule(&config, &new_schedule).map(|_| {
        debug!("ToU schedule set successfully from:{} - to:{} with charge rate: {}W", start, stop, threshold_p_max);
    })
}

pub fn clear_tou_schedule(config: &Config) -> Result<(), Box<dyn Error>> {
    let new_schedule = TouSchedule {
        entries: vec![],
    };

    send_tou_shedule(&config, &new_schedule).map(|_| {
        debug!("ToU schedule cleared successfully.");
    })
}

pub fn send_tou_shedule(config: &Config, new_schedule: &TouSchedule) -> Result<(), Box<dyn Error>> {
    let url = format!("http://{}/api/v2", config.battery.ip);

    let _put_res = ureq::put(&url)
        .header("Auth-Token", &config.battery.token)
        .content_type("application/json")
        .send_json(&new_schedule) // Pass by reference
        .map_err(|e| format!("Failed to send PUT: {}", e))?;
    Ok(())
}

/// Cleanup function to run at exit
pub fn cleanup(config: &Config) {
    info!("Running cleanup procedures...");

    // Clear the ToU schedule on shutdown
    match clear_tou_schedule(config) {
        Ok(_) => info!("Successfully cleared ToU schedule on shutdown"),
        Err(e) => warn!("Failed to clear ToU schedule on shutdown: {}", e),
    }

    info!("Cleanup complete. Shutting down gracefully.");
}
