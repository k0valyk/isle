use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
struct OpenMeteo {
    current: Current,
    hourly: Hourly,
    daily: Daily,
    #[serde(default)]
    timezone: Option<String>,
}

#[derive(Deserialize)]
struct Current {
    time: String,
    temperature_2m: f64,
    relative_humidity_2m: Option<f64>,
    apparent_temperature: Option<f64>,
    is_day: Option<i64>,
    precipitation: Option<f64>,
    weather_code: i64,
    pressure_msl: Option<f64>,
    wind_speed_10m: Option<f64>,
    wind_direction_10m: Option<f64>,
}

#[derive(Deserialize)]
struct Hourly {
    time: Vec<String>,
    temperature_2m: Vec<Option<f64>>,
    weather_code: Vec<Option<i64>>,
    precipitation_probability: Vec<Option<f64>>,
}

#[derive(Deserialize)]
struct Daily {
    time: Vec<String>,
    weather_code: Vec<Option<i64>>,
    temperature_2m_max: Vec<Option<f64>>,
    temperature_2m_min: Vec<Option<f64>>,
    sunrise: Vec<Option<String>>,
    sunset: Vec<Option<String>>,
    uv_index_max: Vec<Option<f64>>,
    precipitation_probability_max: Vec<Option<f64>>,
}

fn value_at<T: Clone>(list: &[Option<T>], index: usize) -> Option<T> {
    list.get(index).cloned().flatten()
}

#[tauri::command]
pub async fn fetch_weather(lat: f64, lon: f64, units: String) -> Result<Value, String> {
    let unit = if units.eq_ignore_ascii_case("f") {
        "fahrenheit"
    } else {
        "celsius"
    };

    let response = reqwest::Client::new()
        .get("https://api.open-meteo.com/v1/forecast")
        .query(&[
            ("latitude", format!("{lat}")),
            ("longitude", format!("{lon}")),
            (
                "current",
                "temperature_2m,relative_humidity_2m,apparent_temperature,is_day,precipitation,weather_code,pressure_msl,wind_speed_10m,wind_direction_10m".to_string(),
            ),
            (
                "hourly",
                "temperature_2m,weather_code,precipitation_probability".to_string(),
            ),
            (
                "daily",
                "weather_code,temperature_2m_max,temperature_2m_min,sunrise,sunset,uv_index_max,precipitation_probability_max".to_string(),
            ),
            ("timezone", "auto".to_string()),
            ("forecast_days", "7".to_string()),
            ("wind_speed_unit", "ms".to_string()),
            ("temperature_unit", unit.to_string()),
        ])
        .timeout(std::time::Duration::from_secs(12))
        .send()
        .await
        .map_err(|error| format!("Open-Meteo request failed: {error}"))?;

    if !response.status().is_success() {
        return Err(format!("Open-Meteo HTTP {}", response.status()));
    }

    let data: OpenMeteo = response
        .json()
        .await
        .map_err(|error| format!("Open-Meteo response is invalid: {error}"))?;

    let hourly: Vec<Value> = data
        .hourly
        .time
        .iter()
        .enumerate()
        .take(48)
        .map(|(index, time)| {
            json!({
                "time": time,
                "temperature": value_at(&data.hourly.temperature_2m, index).unwrap_or(0.0),
                "code": value_at(&data.hourly.weather_code, index).unwrap_or(0),
                "precipitation_probability": value_at(&data.hourly.precipitation_probability, index).unwrap_or(0.0),
            })
        })
        .collect();

    let daily: Vec<Value> = data
        .daily
        .time
        .iter()
        .enumerate()
        .take(7)
        .map(|(index, date)| {
            json!({
                "date": date,
                "code": value_at(&data.daily.weather_code, index).unwrap_or(0),
                "tmax": value_at(&data.daily.temperature_2m_max, index).unwrap_or(0.0),
                "tmin": value_at(&data.daily.temperature_2m_min, index).unwrap_or(0.0),
                "sunrise": value_at(&data.daily.sunrise, index).unwrap_or_default(),
                "sunset": value_at(&data.daily.sunset, index).unwrap_or_default(),
                "uv": value_at(&data.daily.uv_index_max, index).unwrap_or(0.0),
                "precipitation_probability": value_at(&data.daily.precipitation_probability_max, index).unwrap_or(0.0),
            })
        })
        .collect();

    Ok(json!({
        "timezone": data.timezone.unwrap_or_else(|| "UTC".into()),
        "current": {
            "time": data.current.time,
            "temperature": data.current.temperature_2m,
            "apparent": data.current.apparent_temperature.unwrap_or(data.current.temperature_2m),
            "humidity": data.current.relative_humidity_2m.unwrap_or(0.0),
            "wind": data.current.wind_speed_10m.unwrap_or(0.0),
            "wind_direction": data.current.wind_direction_10m.unwrap_or(0.0),
            "code": data.current.weather_code,
            "is_day": data.current.is_day.unwrap_or(1) == 1,
            "precipitation": data.current.precipitation.unwrap_or(0.0),
            "pressure": data.current.pressure_msl.unwrap_or(0.0),
        },
        "hourly": hourly,
        "daily": daily,
    }))
}

#[derive(Deserialize)]
struct GeoResult {
    name: String,
    #[serde(default)]
    country: Option<String>,
    #[serde(default)]
    admin1: Option<String>,
    latitude: f64,
    longitude: f64,
}

#[derive(Deserialize)]
struct GeoResponse {
    #[serde(default)]
    results: Vec<GeoResult>,
}

#[tauri::command]
pub async fn geocode_city(query: String) -> Result<Value, String> {
    if query.trim().len() < 2 {
        return Ok(json!([]));
    }

    let response = reqwest::Client::new()
        .get("https://geocoding-api.open-meteo.com/v1/search")
        .query(&[
            ("name", query.trim().to_string()),
            ("count", "8".to_string()),
            ("language", "uk".to_string()),
            ("format", "json".to_string()),
        ])
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|error| format!("Geocoding request failed: {error}"))?;

    if !response.status().is_success() {
        return Err(format!("Geocoding HTTP {}", response.status()));
    }

    let data: GeoResponse = response
        .json()
        .await
        .map_err(|error| format!("Geocoding response is invalid: {error}"))?;

    let results: Vec<Value> = data
        .results
        .into_iter()
        .map(|city| {
            json!({
                "name": city.name,
                "country": city.country.unwrap_or_default(),
                "admin1": city.admin1,
                "latitude": city.latitude,
                "longitude": city.longitude,
            })
        })
        .collect();

    Ok(json!(results))
}
