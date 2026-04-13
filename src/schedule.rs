use crate::config::Config;
use chrono::{Local, NaiveTime};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::error::Error;

pub trait BatteryClient {
    fn get_schedule(&self) -> Result<TouSchedule, Box<dyn Error>>;
    fn set_schedule(&self, schedule: &TouSchedule) -> Result<(), Box<dyn Error>>;
    fn set_operating_mode(&self, mode: &OperatingMode) -> Result<(), Box<dyn Error>>;
}

pub struct SonnenClient {
    pub ip: String,
    pub token: String,
    pub dry_run: bool, // Add this field to control dry run behavior
}

impl BatteryClient for SonnenClient {
    fn get_schedule(&self) -> Result<TouSchedule, Box<dyn Error>> {
        if self.dry_run {
            let dummy_schedule = TouSchedule {
                entries: vec![],
            };
            Ok(dummy_schedule)
        } else {
            let url = format!("{}/EM_ToU_Schedule", self.base_url());
            let mut response = ureq::get(&url)
                .header("Auth-Token", &self.token)
                .call()?;
            Ok(response.body_mut().read_json()?)
        }
    }

    fn set_schedule(&self, schedule: &TouSchedule) -> Result<(), Box<dyn Error>> {
        // For debugging: print the schedule being sent
        if self.dry_run {
            // Serialize the struct to a pretty-printed string instead of sending it
            serde_json::to_string_pretty(&schedule)
                .map(|json_str| {
                    info!("[DRY RUN] Target: {}", self.ip);
                    info!("[DRY RUN] Would have sent the following JSON:\n{}", json_str);
                })
                .map_err(|e| {
                    error!("[DRY RUN] Failed to serialize JSON: {}", e);
                    e
                })?;
            Ok(())
        } else {
            // Your actual ureq::put(...).send_json(&new_schedule) logic
            let url = self.base_url();
            ureq::put(&url)
                .header("Auth-Token", &self.token)
                .content_type("application/json")
                .send_json(schedule)
                .map_err(|e| format!("Failed to send PUT: {}", e))?;
            Ok(())
        }
    }

    fn set_operating_mode(&self, mode: &OperatingMode) -> Result<(), Box<dyn Error>> {
        // For debugging: print the schedule being sent
        let msg = OperatingModeMsg {
            mode: mode.as_str().to_string(),
        };
        if self.dry_run {
            // Serialize the struct to a pretty-printed string instead of sending it
            serde_json::to_string_pretty(&msg)
                .map(|json_str| {
                    info!("[DRY RUN] Target: {}", self.ip);
                    info!("[DRY RUN] Would have sent the following JSON:\n{}", json_str);
                })
                .map_err(|e| {
                    error!("[DRY RUN] Failed to serialize JSON: {}", e);
                    e
                })?;
            Ok(())
        } else {
            // Your actual ureq::put(...).send_json(&new_schedule) logic
            let url = self.base_url();
            ureq::put(&url)
                .header("Auth-Token", &self.token)
                .content_type("application/json")
                .send_json(msg)
                .map_err(|e| format!("Failed to send PUT: {}", e))?;
            Ok(())
        }

    }
}

impl SonnenClient {
    fn base_url(&self) -> String {
        format!("http://{}/api/v2/configurations", self.ip)
    }
}

pub enum OperatingMode {
    #[allow(dead_code)]
    Manual,
    Auto,
    ToU,
}


impl OperatingMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            OperatingMode::Manual => "1",
            OperatingMode::Auto => "2",
            OperatingMode::ToU => "10",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct OperatingModeMsg {
    #[serde(rename = "EM_OperatingMode")]
    pub mode: String,          // e.g., "2"
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TouEntry {
    pub start: String,          // e.g., "11:00"
    pub stop: String,           // e.g., "14:00"
    pub threshold_p_max: i32,   // 5000 for max charge on Evo
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TouSchedule {
    #[serde(rename = "EM_ToU_Schedule")]
    pub entries: Vec<TouEntry>,
}

pub fn manage_battery(config: &Config, client: &dyn BatteryClient) -> Result<(), Box<dyn Error>> {
    let now = Local::now().time();
    let free_start = config.schedule.start_time.parse::<NaiveTime>()
        .expect("Invalid start_time format in config");
    let free_end = config.schedule.stop_time.parse::<NaiveTime>()
        .expect("Invalid start_time format in config");

    // 1. Get current config from Battery
    let current_schedule = client.get_schedule()?;

    // 2. Determine what SHOULD be happening
    let is_free_window = now >= free_start && now < free_end;

    if is_free_window && current_schedule.entries.is_empty() {
        let new_schedule = make_schedule(config, config.battery.max_charge_watt);
        debug!("Current schedule is empty and we are in the free window. Setting new schedule:");
        if &new_schedule == &current_schedule {
            debug!("Schedule already set correctly.");
        } else {
            set_tou_schedule(client, &new_schedule)? // 5kW charge
        }
    } else if !is_free_window && !current_schedule.entries.is_empty() {
        // ACTION: Free time is over. Clear the schedule.
        clear_tou_schedule(config, client)? // Sends []
    }
    Ok(())
}

pub fn make_schedule(config: &Config, threshold_p_max: i32) -> TouSchedule {
    let start = config.schedule.start_time.clone();
    let stop = config.schedule.stop_time.clone();
    TouSchedule {
        entries: vec![TouEntry {
            start: start.clone(),
            stop: stop.clone(),
            threshold_p_max: threshold_p_max,
        }],
    }
}
pub fn set_tou_schedule(client: &dyn BatteryClient, new_schedule: &TouSchedule) -> Result<(), Box<dyn Error>> {
    client.set_operating_mode(&OperatingMode::ToU).map(|_| {
        debug!("Operating mode set to TimeOfUse successfully.");
    }).and_then(|_| {
        client.set_schedule(new_schedule).map(|_| {
            debug!("ToU schedule set successfully to {:?}.", new_schedule);
        })
    })
}

pub fn clear_tou_schedule(_config: &Config, client: &dyn BatteryClient) -> Result<(), Box<dyn Error>> {
    let new_schedule = TouSchedule {
        entries: vec![],
    };

    client.set_operating_mode(&OperatingMode::Auto).map(|_| {
        debug!("Operating mode set to Auto successfully.");
    }).and_then(|_| {
        client.set_schedule(&new_schedule).map(|_| {
            debug!("ToU schedule cleared successfully.");
        })
    })
}

/// Cleanup function to run at exit
pub fn cleanup(config: &Config, client: &dyn BatteryClient) {
    info!("Running cleanup procedures...");

    // Clear the ToU schedule on shutdown
    match clear_tou_schedule(config, client) {
        Ok(_) => info!("Successfully cleared ToU schedule on shutdown"),
        Err(e) => warn!("Failed to clear ToU schedule on shutdown: {}", e),
    }

    info!("Cleanup complete. Shutting down gracefully.");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BatteryConfig, ScheduleConfig};
    use std::sync::Mutex;

    struct MockBattery {
        current_schedule: Mutex<TouSchedule>,
        set_calls: Mutex<Vec<TouSchedule>>,
    }

    impl MockBattery {
        fn new(initial_schedule: TouSchedule) -> Self {
            Self {
                current_schedule: Mutex::new(initial_schedule),
                set_calls: Mutex::new(Vec::new()),
            }
        }
    }

    impl BatteryClient for MockBattery {
        fn get_schedule(&self) -> Result<TouSchedule, Box<dyn Error>> {
            Ok(self.current_schedule.lock().unwrap().clone())
        }

        fn set_schedule(&self, schedule: &TouSchedule) -> Result<(), Box<dyn Error>> {
            let json = serde_json::to_string_pretty(schedule).unwrap_or_else(|_| "Invalid JSON".to_string());
            println!("--- [MOCK BATTERY] SET SCHEDULE ---\n{}\n----------------------------------", json);
            self.set_calls.lock().unwrap().push(schedule.clone());
            *self.current_schedule.lock().unwrap() = schedule.clone();
            Ok(())
        }

        fn set_operating_mode(&self, _mode: &OperatingMode) -> Result<(), Box<dyn Error>> {
            Ok(())
        }
    }

    fn create_test_config(start: &str, stop: &str) -> Config {
        Config {
            battery: BatteryConfig {
                ip: "127.0.0.1".to_string(),
                token: "test-token".to_string(),
                max_charge_watt: 5000,
            },
            schedule: ScheduleConfig {
                start_time: start.to_string(),
                stop_time: stop.to_string(),
            },
        }
    }

    #[test]
    fn test_manage_battery_sets_schedule_when_in_window_and_empty() {
        let config = create_test_config("00:00", "23:59"); // Always in window
        let initial_schedule = TouSchedule { entries: vec![] };
        let mock = MockBattery::new(initial_schedule);

        manage_battery(&config, &mock).unwrap();

        let calls = mock.set_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].entries.len(), 1);
        assert_eq!(calls[0].entries[0].threshold_p_max, 5000);
    }

    #[test]
    fn test_manage_battery_clears_schedule_when_out_of_window_and_not_empty() {
        let config = create_test_config("23:58", "23:59"); // Likely out of window
        // If we happen to run this at exactly 23:58:xx, this test might fail. 
        // In a real scenario, we'd mock the time too, but for "simple mocks" this is a start.
        
        let initial_schedule = TouSchedule {
            entries: vec![TouEntry {
                start: "23:58".to_string(),
                stop: "23:59".to_string(),
                threshold_p_max: 5000,
            }],
        };
        let mock = MockBattery::new(initial_schedule);

        // We need to ensure we are actually out of window for this test to be reliable.
        // Let's use a time that is definitely not now.
        // Since we can't easily mock Local::now() without changing more code, 
        // we'll just check if it's currently NOT in the window.
        let now = Local::now().time();
        let free_start = NaiveTime::from_hms_opt(23, 58, 0).unwrap();
        let free_end = NaiveTime::from_hms_opt(23, 59, 0).unwrap();
        let is_free_window = now >= free_start && now < free_end;

        if !is_free_window {
            manage_battery(&config, &mock).unwrap();
            let calls = mock.set_calls.lock().unwrap();
            assert_eq!(calls.len(), 1);
            assert!(calls[0].entries.is_empty());
        }
    }
}
