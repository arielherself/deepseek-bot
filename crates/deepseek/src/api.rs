use crate::types::*;

pub struct DeepSeekAPI {
    pub token: String,
    pub timeout: u64,
}

impl DeepSeekAPI {
    pub async fn get_balance(&self) -> Result<String, Box<dyn std::error::Error + Sync + Send>> {
        let client = reqwest::Client::new();
        let response = client.get("https://api.deepseek.com/user/balance")
            .timeout(std::time::Duration::from_millis(self.timeout))
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?;
        let payload = serde_json::from_str::<DeepSeekUserBalance>(response.text().await?.as_str())?;
        let mut ret = format!("Available: {}\n", payload.is_available);
        for info in payload.balance_infos {
            ret.push_str(&format!("  Currency: {}\n  Total Balance: {}\n\n", info.currency, info.total_balance));
        }
        Ok(ret)
    }
    pub async fn single_message_dialog(&self, query: String) -> Result<String, Box<dyn std::error::Error + Sync + Send>> {
        let client = reqwest::Client::new();
        let json_body = format!(r#"{{
            "model": "deepseek-chat",
            "messages": [
              {{"role": "user", "content": "{}"}}
            ],
            "stream": false
        }}"#, query);
        let response = client.post("https://api.deepseek.com/chat/completions")
            .timeout(std::time::Duration::from_millis(self.timeout))
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.token))
            .body(json_body.to_owned())
            .send()
            .await?;
        let payload = serde_json::from_str::<DeepSeekChatResponse>(response.text().await?.as_str())?;
        let mut ret = String::from("DeepSeek didn't provide any valid response to your query.");
        if payload.choices.len() > 0 {
            if let Some(text) = &payload.choices[0].message.content {
                ret = text.as_str().to_string()
            }
        }
        Ok(ret)
    }
}


