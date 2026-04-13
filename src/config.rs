use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub battery: BatteryConfig,
    pub schedule: ScheduleConfig,
}

#[derive(Deserialize)]
pub struct BatteryConfig {
    pub ip: String,
    pub token: String,
    pub max_charge_watt: i32,
}

#[derive(Deserialize)]
pub struct ScheduleConfig {
    pub start_time: String,
    pub stop_time: String,
}
