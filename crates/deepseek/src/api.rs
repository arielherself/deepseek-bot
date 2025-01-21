use crate::types::*;

use std::fmt::Write;
fn report(mut err: &dyn std::error::Error) -> String {
    let mut s = format!("{}", err);
    while let Some(src) = err.source() {
        let _ = write!(s, "\n\nCaused by: {}", src);
        err = src;
    }
    s
}

#[derive(Clone)]
pub struct DeepSeekAPI {
    pub token: String,
    pub timeout: u64,
    pub client: reqwest::Client,
}

impl DeepSeekAPI {
    pub async fn get_balance(&self) -> Result<String, Box<dyn std::error::Error + Sync + Send>> {
        let response = self.client.get("https://api.deepseek.com/user/balance")
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
    pub async fn single_message_dialog(&self, max_tokens: u64, query: String, model: DeepSeekModel) -> Result<String, Box<dyn std::error::Error + Sync + Send>> {
        self.single_message_dialog_with_system(max_tokens, query, String::new(), model).await
    }
    pub async fn single_message_dialog_with_system(&self, max_tokens: u64, query: String, system: String, model: DeepSeekModel) -> Result<String, Box<dyn std::error::Error + Sync + Send>> {
        let model_name = match model {
            DeepSeekModel::DeepSeekChat => "deepseek-chat",
            DeepSeekModel::DeepSeekReasoner => "deepseek-reasoner",
        };
        let json_body = format!(r#"{{
            "model": "{}",
            "max_tokens": {},
            "messages": [
              {{"role": "system", "content": {}}},
              {{"role": "user", "content": {}}}
            ],
            "stream": false
        }}"#, model_name, max_tokens, serde_json::Value::String(system).to_string(), serde_json::Value::String(query).to_string());
        eprintln!("{json_body}");
        let response = match self.client.post("https://api.deepseek.com/chat/completions")
            .timeout(std::time::Duration::from_millis(self.timeout))
            .header("User-Agent", "PostmanRuntime/7.43.0")
            .header("Cookie", "HWWAFSESID=a8e7a20b4e490a972ef; HWWAFSESTIME=1735732935007")
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {}", self.token))
            .body(json_body.to_owned())
            .send()
            .await {
                Ok(response) => response,
                Err(e) => {
                    eprintln!("ERROR: {}", report(&e));
                    return Err(Box::new(e));
                }
            };
        let payload = serde_json::from_str::<DeepSeekChatResponse>(response.text().await?.as_str())?;
        let mut ret = String::from("DeepSeek didn't provide any valid response to your query.");
        if payload.choices.len() > 0 {
            if let Some(text) = &payload.choices[0].message.content {
                eprintln!("{}", text.as_str().to_string());
                ret = text.as_str().to_string()
            }
        }
        Ok(ret)
    }
}


