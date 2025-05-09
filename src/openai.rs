use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
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