use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NewUiStruct<'a> {
    pub ts: &'a str,
    pub stat_state: &'a str,
    pub gui_bat_data_power: &'a str,
    pub gui_inverter_power: &'a str,
    pub gui_house_pow: &'a str,
    pub gui_grid_pow: &'a str,
    pub gui_bat_data_fuel_charge: &'a str,
    pub gui_charging_info: &'a str,
    pub gui_boosting_info: &'a str,
    pub weather: WeatherNew,
    pub total_data: TotalDataNew<'a>,
    pub prediction: Option<Prediction /*<'a>*/>,
}
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Prediction {
    pub prediction: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TotalDataNew<'a> {
    pub consumption: &'a str,
    pub generated: &'a str,
    pub new: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WeatherNew {
    pub hourly: HourlyNew,
    pub daily: DailyNew,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DailyNew {
    pub time: Vec<String>,
    pub sunset: Vec<String>,
    pub sunrise: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HourlyNew {
    pub time: Vec<String>,
    #[serde(rename = "temperature_2m")]
    pub temperature_2_m: Vec<String>,
    pub rain: Vec<String>,
    pub showers: Vec<String>,
    pub cloud_cover: Vec<String>,
    pub uv_index: Vec<String>,
    pub uv_index_clear_sky: Vec<String>,
}
