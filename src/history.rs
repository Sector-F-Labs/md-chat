//! Chat history from a local [Reservoir](https://github.com/Sector-F-Labs/reservoir).

use crate::app::ChatMessage;

pub async fn fetch() -> Result<Vec<ChatMessage>, String> {
    let url = "http://localhost:3017/partition/default/instance/default/command/view/15";
    let response = reqwest::Client::new().get(url).send().await.map_err(|e| e.to_string())?;
    let text = response.text().await.map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}
