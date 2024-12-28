use serde::Deserialize;
use serde::Serialize;
use teloxide::payloads::SendMessageSetters;
use teloxide::RequestError;
use teloxide::types::*;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

#[derive(Deserialize)]
struct Config {
    telegram_bot_token: String,
    deepseek_api_token: String,
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "display this text")]
    Help,
    #[command(description = "test connectivity")]
    Die,
}

#[derive(Deserialize)]
struct DeepSeekCompletionProbabilityTop {
    token: String,
    logprob: i64,
    bytes: Option<Vec<u8>>,
}

#[derive(Deserialize)]
struct DeepSeekCompletionProbabilityInfo {
    token: String,
    logprob: i64,
    bytes: Option<Vec<u8>>,
    top_logprobs: Vec<DeepSeekCompletionProbabilityTop>,
}

#[derive(Deserialize)]
struct DeepSeekCompletionProbability {
    content: Option<Vec<DeepSeekCompletionProbabilityInfo>>,
}

#[derive(Deserialize)]
struct DeepSeekCompletionMessageToolCallFunction {
    name: String,
    arguments: String,
}

#[derive(Deserialize)]
struct DeepSeekCompletionMessageToolCall {
    id: String,
    function: DeepSeekCompletionMessageToolCallFunction,
}

#[derive(Deserialize)]
struct DeepSeekCompletionMessage {
    content: Option<String>,
    tool_calls: Option<Vec<DeepSeekCompletionMessageToolCall>>,
    role: String,
}

#[derive(Deserialize)]
struct DeepSeekCompletionChoice {
    finish_reason: String,
    index: u64,
    message: DeepSeekCompletionMessage,
    logprobs: Option<DeepSeekCompletionProbability>,
}

/// ref: https://api-docs.deepseek.com/api/create-chat-completion
#[derive(Deserialize)]
struct DeepSeekChatResponse {
    id: String,
    choices: Vec<DeepSeekCompletionChoice>,
    created: u64,
    model: String,
    system_fingerprint: String,
    object: String,
}

struct DeepSeekAPI {
    token: String,
}

impl DeepSeekAPI {
    async fn single_message_dialog(&self, query: String) -> Result<String, Box<dyn std::error::Error + Sync + Send>> {
        let client = reqwest::Client::new();
        let json_body = format!(r#"{{
            "model": "deepseek-chat",
            "messages": [
              {{"role": "user", "content": "{}"}}
            ],
            "stream": false
        }}"#, query);
        log::debug!("Sending request with body = {}", json_body);
        let response = client.post("https://api.deepseek.com/chat/completions")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.token))
            .body(json_body)
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


async fn reply_to_message(bot: Bot, msg: Message, text: String) -> Result<String, RequestError> {
    let reply_parameters = ReplyParameters {
        message_id: msg.id,
        chat_id: None,
        allow_sending_without_reply: Some(false),
        quote: None,
        quote_parse_mode: None,
        quote_entities: None,
        quote_position: None,
    };
    const MAX_RETRY: usize = 3;
    for i in 0..MAX_RETRY {
        match bot.send_message(
            msg.chat.id,
            text.replace(".", "\\.")
                .replace("-", "\\-")
                .replace(">", "\\>")
                .replace("<", "\\<")
                .replace("{", "\\{")
                .replace("}", "\\}")
                .replace("=", "\\=")
                .replace("#", "\\#")
                ).parse_mode(ParseMode::MarkdownV2)
                .reply_parameters(reply_parameters.clone())
                .await {
            Ok(response) => {
                return Ok(response.text().unwrap_or("[void]").to_string())
            },
            Err(e) => {
                if i == MAX_RETRY - 1 {
                    return Err(e);
                }
            }
        }
    }
    Ok(String::from("[void]"))
}

async fn chat_handler(bot: Bot, msg: Message, api: DeepSeekAPI) -> ResponseResult<()> {
    log::debug!("called chat_handler");
    match msg.text() {
        Some(text) => {
            log::debug!("Received msg = {}", text);
            let mut response = String::from("You are seeing this message because there was an error when we communicate with DeepSeek. Check the log for details.");
            match api.single_message_dialog(String::from(text)).await {
                Ok(reply) => {
                    response = reply;
                },
                Err(e) => {
                    log::error!("Unable to get response from DeepSeek: {}", e);
                },
            }
            match reply_to_message(bot, msg, response).await {
                Ok(msg) => log::debug!("sent response = {}", msg),
                Err(e) => log::error!("error sending response: {}", e),
            }
        },
        None => (),
    };

    Ok(())
}

async fn command_handler(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            reply_to_message(bot, msg, Command::descriptions().to_string()).await?;
        }
        Command::Die => {
            bot.send_dice(msg.chat.id).await?;
        }
    };

    Ok(())
}

async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    let config = toml::from_str::<Config>(std::fs::read_to_string("config.toml")?.as_str())?;
    log::info!("Read bot token = {}", config.telegram_bot_token);

    let bot = Bot::new(config.telegram_bot_token);
    let deepseek_api_token = config.deepseek_api_token;
    Dispatcher::builder(bot, dptree::entry()
        .branch(
            Update::filter_message().filter_command::<Command>().endpoint(command_handler),
        ).branch(
            Update::filter_message().endpoint(move |bot: Bot, msg: Message| {
                let deepseek_api_token = deepseek_api_token.clone();
                async move {
                    chat_handler(bot, msg, DeepSeekAPI { token: deepseek_api_token }).await
                }
            }),
        )
    ).enable_ctrlc_handler().build().dispatch().await;

    Ok(())
}

#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Debug).unwrap();
    log::info!("Service started.");

    if let Err(e) = serve().await {
        log::error!("{}", e);
    }

    log::info!("Service stopped.");
}
