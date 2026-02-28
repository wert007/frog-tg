use anyhow::bail;
use chrono::Local;

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
    Mist,
    Snow,
    Graupel,
    Unknown,
}
impl Percipation {
    fn from_omr(omr: OpenMeteoResponse) -> Self {
        if omr.rain < 0.2 {
            if omr.snowfall < 0.2 {
                Self::None
            } else {
                Self::Snow
            }
        } else if omr.rain < 0.5 {
            Self::Drizzle
        } else if omr.rain < 5.0 {
            Self::ModerateRain
        } else {
            Self::StrongRain
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
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
struct OpenMeteoResponse {
    time: chrono::DateTime<Local>,
    interval: i64,
    temperature_2m: f64,
    relative_humidity_2m: i64,
    wind_speed_10m: f64,
    cloud_cover: f64,
    rain: f64,
    snowfall: f64,
    weather_code: u8,
}

impl WeatherStats {
    pub async fn current() -> anyhow::Result<Self> {
        let a: serde_json::Value = reqwest::get(OPENMETEO_URL)
            .await?
            .error_for_status()?
            .json()
            .await?;
        let Some(a) = a.as_object().map(|a| a.get("current")).flatten() else {
            bail!("Unexpected response from openmeteo")
        };
        let omr: OpenMeteoResponse = serde_json::from_value(a.clone())?;
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
        })
    }
}
