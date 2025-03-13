use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime, PrimitiveDateTime};
use tokio::sync::RwLock;

#[derive(Serialize, Clone, Debug)]
pub struct DateInfoEventMode {
    pub time: OffsetDateTime,
    pub name: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct DateInfo {
    pub date: Date,
    pub holiday: Option<String>,
    pub events: HashMap<String, DateInfoEventMode>,
}

pub type CalendarMap = BTreeMap<Date, DateInfo>;
pub type CalendarMapArc = Arc<RwLock<CalendarMap>>;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum WeatherInfoState {
    ClearNight,
    Cloudy,
    Exceptional,
    Fog,
    Hail,
    Lightning,
    LightningRainy,
    Partlycloudy,
    Pouring,
    Rainy,
    Snowy,
    SnowyRainy,
    Sunny,
    Windy,
    WindyVariant,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WeatherInfoAttribute {
    pub temperature: f32,
    pub dew_point: f32,
    pub temperature_unit: String,
    pub humidity: f32,
    pub cloud_coverage: f32,
    pub uv_index: f32,
    pub pressure: f32,
    pub wind_bearing: f32,
    pub wind_speed: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WeatherInfo {
    pub state: WeatherInfoState,
    pub attributes: WeatherInfoAttribute,
}

pub type WeatherInfoArc = Arc<RwLock<Option<WeatherInfo>>>;

// * Route mode

#[derive(Deserialize, Default, Debug, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum QueryRouteEPaperOutputEnum {
    #[default]
    Full,
    Black,
    BlackInvert,
    Red,
}

#[derive(Deserialize, Default, Debug, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum QueryRouteEPaperFormatEnum {
    #[default]
    Png,
    Bmp,
}

#[derive(Deserialize, Default)]
pub struct QueryRouteEPaperModel {
    #[serde(default)]
    pub output: QueryRouteEPaperOutputEnum,
    #[serde(default)]
    pub format: QueryRouteEPaperFormatEnum,
}

pub type LastUpdateArc = Arc<RwLock<PrimitiveDateTime>>;
