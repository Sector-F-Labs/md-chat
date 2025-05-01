use serde::{Deserialize, Serialize};
use std::env;

const DEFAULT_API_URL: &str = "https://api.openai.com/v1/chat/completions";

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

pub async fn send_openai_request(message: &str, model: &str) -> Result<String, String> {
    let api_key = env::var("OPENAI_API_KEY")
        .map_err(|_| "OPENAI_API_KEY environment variable not set")?;
    
    let api_url = env::var("OPENAI_API_URL").unwrap_or_else(|_| DEFAULT_API_URL.to_string());

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