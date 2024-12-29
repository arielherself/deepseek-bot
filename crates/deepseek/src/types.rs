use serde::Deserialize;

#[derive(Deserialize)]
pub struct DeepSeekCompletionProbabilityTop {
    pub token: String,
    pub logprob: i64,
    pub bytes: Option<Vec<u8>>,
}

#[derive(Deserialize)]
pub struct DeepSeekCompletionProbabilityInfo {
    pub token: String,
    pub logprob: i64,
    pub bytes: Option<Vec<u8>>,
    pub top_logprobs: Vec<DeepSeekCompletionProbabilityTop>,
}

#[derive(Deserialize)]
pub struct DeepSeekCompletionProbability {
    pub content: Option<Vec<DeepSeekCompletionProbabilityInfo>>,
}

#[derive(Deserialize)]
pub struct DeepSeekCompletionMessageToolCallFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Deserialize)]
pub struct DeepSeekCompletionMessageToolCall {
    pub id: String,
    pub function: DeepSeekCompletionMessageToolCallFunction,
}

#[derive(Deserialize)]
pub struct DeepSeekCompletionMessage {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<DeepSeekCompletionMessageToolCall>>,
    pub role: String,
}

#[derive(Deserialize)]
pub struct DeepSeekCompletionChoice {
    pub finish_reason: String,
    pub index: u64,
    pub message: DeepSeekCompletionMessage,
    pub logprobs: Option<DeepSeekCompletionProbability>,
}

/// ref: https://api-docs.deepseek.com/api/create-chat-completion
#[derive(Deserialize)]
pub struct DeepSeekChatResponse {
    pub id: String,
    pub choices: Vec<DeepSeekCompletionChoice>,
    pub created: u64,
    pub model: String,
    pub system_fingerprint: String,
    pub object: String,
}

#[derive(Deserialize)]
pub struct DeepSeekUserBalanceInfo {
    pub currency: String,
    pub total_balance: String,
    pub granted_balance: String,
    pub topped_up_balance: String,
}

#[derive(Deserialize)]
pub struct DeepSeekUserBalance {
    pub is_available: bool,
    pub balance_infos: Vec<DeepSeekUserBalanceInfo>,
}



