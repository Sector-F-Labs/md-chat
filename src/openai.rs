use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    #[default]
    User,
    Assistant,
}

#[derive(Debug, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatCompletionMessage>,
    pub temperature: f32,
}

#[derive(Debug, Serialize)]
pub struct ChatCompletionMessage {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatCompletionResponse {
    pub choices: Vec<ChatCompletionChoice>,
}

#[derive(Debug, Deserialize)]
pub struct ChatCompletionChoice {
    pub message: ChatCompletionMessageResponse,
}

#[derive(Debug, Deserialize)]
pub struct ChatCompletionMessageResponse {
    pub content: String,
}

pub async fn send_openai_request(
    message: &str,
    model: &str,
    api_key: &str,
    api_url: &str,
) -> Result<String, String> {
    let client = reqwest::Client::new();
    let request = ChatCompletionRequest {
        model: model.to_string(),
        messages: vec![
            ChatCompletionMessage {
                role: Role::System,
                content: "You are a helpful assistant. You can use markdown formatting in your responses.".to_string(),
            },
            ChatCompletionMessage {
                role: Role::User,
                content: message.to_string(),
            },
        ],
        temperature: 0.7,
    };

    let response = client
        .post(api_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let completion: ChatCompletionResponse = response
        .json()
        .await
        .map_err(|e| e.to_string())?;

    completion
        .choices
        .first()
        .map(|choice| choice.message.content.clone())
        .ok_or_else(|| "No response from OpenAI".to_string())
} 
#[derive(Debug, Deserialize)]
struct ModelList {
    data: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    id: String,
}

/// The models the API has (`GET /v1/models`), leaving out embedding models,
/// which can't chat.
pub async fn list_models(models_url: &str, api_key: &str) -> Result<Vec<String>, String> {
    let mut request = reqwest::Client::new().get(models_url);
    if !api_key.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", api_key));
    }
    let response = request.send().await.map_err(|e| e.to_string())?;
    let response = response.error_for_status().map_err(|e| e.to_string())?;
    let list: ModelList = response.json().await.map_err(|e| e.to_string())?;
    Ok(chat_models(list))
}

fn chat_models(list: ModelList) -> Vec<String> {
    list.data.into_iter().map(|m| m.id).filter(|id| !id.to_lowercase().contains("embed")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedding_models_are_left_out() {
        let list: ModelList = serde_json::from_str(
            r#"{"object":"list","data":[{"id":"qwen3.5:latest"},{"id":"nomic-embed-text:latest"},{"id":"embeddinggemma:latest"},{"id":"gemma4:12b"}]}"#,
        )
        .unwrap();
        assert_eq!(chat_models(list), ["qwen3.5:latest", "gemma4:12b"]);
    }
}
