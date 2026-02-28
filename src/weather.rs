use anyhow::{Context, bail};

pub trait BotWeatherExt: teloxide::prelude::Requester {
    async fn send_weather_stats<C: Into<teloxide::types::Recipient>>(
        &self,
        chat_id: C,
        weather: WeatherStats,
    ) -> Result<(), Self::Err>;
}

impl BotWeatherExt for teloxide::Bot {
    async fn send_weather_stats<C: Into<teloxide::types::Recipient>>(
        &self,
        chat_id: C,
        weather: WeatherStats,
    ) -> Result<(), Self::Err> {
        let text = format!(
            "Temperature: {} °C\nWind: {}\nPercipation: {} (WMO: {})\nCloudiness: {}",
            weather.temperature_start,
            weather.wind_beaufort,
            weather.percipation,
            weather.wmo_code,
            weather.cloudiness
        );
        teloxide::prelude::Requester::send_message(&self, chat_id, text).await?;
        Ok(())
    }
}

const OPENMETEO_URL: &'static str = include_str!("../openmeteo-url.txt").trim_ascii();

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
enum Beaufort {
    Zero,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Higher,
}

impl std::fmt::Display for Beaufort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Beaufort::Zero => write!(f, "0 bft"),
            Beaufort::One => write!(f, "1 bft"),
            Beaufort::Two => write!(f, "2 bft"),
            Beaufort::Three => write!(f, "3 bft"),
            Beaufort::Four => write!(f, "4 bft"),
            Beaufort::Five => write!(f, "5 bft"),
            Beaufort::Six => write!(f, "6 bft"),
            Beaufort::Higher => write!(f, "7+ bft"),
        }
    }
}

impl Beaufort {
    fn from_speed(wind_speed_10m: f64) -> Beaufort {
        match wind_speed_10m {
            0.0..0.3 => Beaufort::Zero,
            0.3..1.6 => Beaufort::One,
            1.6..3.4 => Beaufort::Two,
            3.4..5.5 => Beaufort::Three,
            5.5..8.0 => Beaufort::Four,
            8.0..10.8 => Beaufort::Five,
            10.8..13.9 => Beaufort::Six,
            _ => Beaufort::Higher,
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
enum Percipation {
    None,
    StrongRain,
    ModerateRain,
    Drizzle,
    Fog,
    Snow,
    Graupel,
    Unknown,
}

impl std::fmt::Display for Percipation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Percipation::None => write!(f, "none"),
            Percipation::StrongRain => write!(f, "strong rain"),
            Percipation::ModerateRain => write!(f, "rain"),
            Percipation::Drizzle => write!(f, "drizzle"),
            Percipation::Fog => write!(f, "foggy"),
            Percipation::Snow => write!(f, "snow"),
            Percipation::Graupel => write!(f, "graupel"),
            Percipation::Unknown => write!(f, "?"),
        }
    }
}

impl Percipation {
    fn from_omr(omr: OpenMeteoResponse) -> Self {
        match omr.weather_code {
            0..=9 => Percipation::None,
            20 | 24 | 50..=59 => Percipation::Drizzle,
            21 | 61 | 63 | 66 => Percipation::ModerateRain,
            65 | 67 => Percipation::StrongRain,
            22 | 71 | 73 | 75 => Percipation::Snow,
            23 | 77 => Percipation::Graupel,
            10 | 28 | 42..=49 => Percipation::Fog,
            _ => Percipation::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
enum GroundHumidity {
    Wet,
    Humid,
    Dry,
    VeryDry,
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
enum Cloudiness {
    AllClouds,
    ManyClouds,
    Clouds,
    FewClouds,
    Clear,
    GettingCloudy,
    GettingClear,
    Error(f64),
}

impl std::fmt::Display for Cloudiness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Cloudiness::AllClouds => write!(f, "all clouds"),
            Cloudiness::ManyClouds => write!(f, "a lot of clouds"),
            Cloudiness::Clouds => write!(f, "clouds"),
            Cloudiness::FewClouds => write!(f, "a few clouds"),
            Cloudiness::Clear => write!(f, "no clouds"),
            Cloudiness::GettingCloudy => write!(f, "getting cloudy"),
            Cloudiness::GettingClear => write!(f, "getting clear"),
            Cloudiness::Error(e) => write!(f, "error {e}"),
        }
    }
}

impl Cloudiness {
    fn from_cloud_cover(cloud_cover: f64) -> Cloudiness {
        match cloud_cover {
            0.0..20.0 => Self::Clear,
            20.0..40.0 => Self::FewClouds,
            40.0..60.0 => Self::Clouds,
            60.0..80.0 => Self::ManyClouds,
            80.0..100.0 => Self::AllClouds,
            err => Self::Error(err),
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub struct WeatherStats {
    temperature_start: f64,
    temperature_end: Option<f64>,
    wind_beaufort: Beaufort,
    percipation: Percipation,
    ground_humidity: Option<GroundHumidity>,
    cloudiness: Cloudiness,
    wmo_code: u8,
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
struct OpenMeteoResponse {
    // TODO: This would be nice but is always a bit of struggle in serde...
    // time: chrono::DateTime<Local>,
    interval: i64,
    temperature_2m: f64,
    wind_speed_10m: f64,
    cloud_cover: f64,
    weather_code: u8,
}

impl WeatherStats {
    pub async fn current() -> anyhow::Result<Self> {
        let a: serde_json::Value = reqwest::get(OPENMETEO_URL)
            .await
            .context("Could not GET <OPENMETEO_URL>")?
            .error_for_status()
            .context("Status was error for GET <OPNEMETEO_URL>")?
            .json()
            .await
            .context("Invalid json from GET <OPENMETEO_URL>")?;
        let Some(a) = a.as_object().map(|a| a.get("current")).flatten() else {
            bail!("Unexpected response from openmeteo")
        };
        let omr: OpenMeteoResponse =
            serde_json::from_value(a.clone()).context("Unexpected response from openmeteo")?;
        let wind_beaufort = Beaufort::from_speed(omr.wind_speed_10m);
        let cloudiness = Cloudiness::from_cloud_cover(omr.cloud_cover);
        let percipation = Percipation::from_omr(omr);
        Ok(Self {
            temperature_start: omr.temperature_2m,
            temperature_end: None,
            wind_beaufort,
            percipation,
            ground_humidity: None,
            cloudiness,
            wmo_code: omr.weather_code,
        })
    }

    pub(crate) async fn ending(&mut self) -> anyhow::Result<()> {
        // TODO: Get temperature at end of walk!
        Ok(())
    }
}
