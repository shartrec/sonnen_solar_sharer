/*
 * Copyright (c) 2026. Trevor Campbell and others.
 *
 * This file is part of sonnen_solar_sharer.
 *
 * sonnen_solar_sharer is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License,or
 * (at your option) any later version.
 *
 * sonnen_solar_sharer is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with sonnen_solar_sharer; if not, write to the Free Software
 * Foundation, Inc., 59 Temple Place, Suite 330, Boston, MA  02111-1307  USA
 *
 * Contributors:
 *      Trevor Campbell
 *
 */

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
