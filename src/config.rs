use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq)]
pub struct Config {
    pub battery: BatteryConfig,
    pub schedule: ScheduleConfig,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct BatteryConfig {
    pub ip: String,
    pub token: String,
    pub max_charge_watt: i32,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct ScheduleConfig {
    pub start_time: String,
    pub stop_time: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_config() {
        let toml_str = r#"
            [battery]
            ip = "192.168.1.100"
            token = "secret-token"
            max_charge_watt = 5000

            [schedule]
            start_time = "11:00"
            stop_time = "13:30"
        "#;

        let config: Config = toml::from_str(toml_str).expect("Failed to parse config");

        assert_eq!(config.battery.ip, "192.168.1.100");
        assert_eq!(config.battery.token, "secret-token");
        assert_eq!(config.battery.max_charge_watt, 5000);
        assert_eq!(config.schedule.start_time, "11:00");
        assert_eq!(config.schedule.stop_time, "13:30");
    }

    #[test]
    fn test_parse_invalid_toml() {
        let toml_str = r#"
            [battery]
            ip = "192.168.1.100"
            # missing token
        "#;

        let result: Result<Config, _> = toml::from_str(toml_str);
        assert!(result.is_err());
    }
}
