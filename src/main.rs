use deepseek::api::DeepSeekAPI;
use serde::Deserialize;
use teloxide::payloads::EditMessageTextInlineSetters;
use teloxide::payloads::SendMessageSetters;
use teloxide::RequestError;
use teloxide::types::*;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

const MAX_RETRY: usize = 3;
const TIMEOUT: u64 = 1000 * 60 * 3;
const MAX_TOKENS: u64 = 2048;

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
    #[command(description = "get account information")]
    Info,
}

macro_rules! retry_future {
    ($future:expr) => {{
        let mut result = $future.await;
        if matches!(result, Err(_)) {
            for _ in 1..MAX_RETRY {
                let new_result = $future.await;
                if matches!(result, Ok(_)) {
                    result = new_result;
                    break;
                }
            }
        }
        result
    }};
}

/// ref:
/// https://github.com/python-telegram-bot/python-telegram-bot/blob/4f255b6e21debd7ff5274400bf0d36e56bf169fa/telegram/helpers.py#L46
fn escape_markdown(text: String) -> String {
    const ESCAPE_CHARS: &str = r"\_*[]()~`>#+-=|{}.!";
    let escaped_pattern = regex::escape(ESCAPE_CHARS);
    let re = regex::Regex::new(&format!("([{}])", escaped_pattern)).unwrap();
    re.replace_all(&text, r"\$1").to_string()
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
    retry_future!(bot.send_message(
            msg.chat.id,
            escape_markdown(text.to_owned()))
        .parse_mode(ParseMode::MarkdownV2)
        .reply_parameters(reply_parameters.clone()))?;
    Ok(String::from("[void]"))
}

fn generate_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup {
        inline_keyboard: vec![vec![
            InlineKeyboardButton {
                text: format!("Try it!"),
                kind: InlineKeyboardButtonKind::SwitchInlineQueryCurrentChat(String::new()),
            }
        ]],
    }
}

async fn inline_handler(bot: Bot, msg: InlineQuery) -> ResponseResult<()> {
    log::debug!("called inline_handler");
    let cand: Vec<InlineQueryResult> = vec![InlineQueryResult::Article(InlineQueryResultArticle {
        id: format!("arielherself"),
        title: format!("Ask a question"),
        input_message_content: InputMessageContent::Text(InputMessageContentText {
            message_text: escape_markdown(msg.query.to_owned()),
            parse_mode: Some(ParseMode::MarkdownV2),
            entities: None,
            link_preview_options: None,
        }),
        reply_markup: Some(generate_keyboard()),
        url: None,
        hide_url: None,
        description: Some(msg.query),
        thumbnail_url: Some(url::Url::parse("https://avatars.githubusercontent.com/u/148330874").unwrap()),
        thumbnail_width: None,
        thumbnail_height: None,
    })];
    retry_future!(bot.answer_inline_query(msg.id.to_owned(), cand.to_owned()))?;
    Ok(())
}

async fn inline_result_handler(bot: Bot, msg: ChosenInlineResult, api: DeepSeekAPI) -> ResponseResult<()> {
    log::debug!("called callback_handler");
    let query = msg.query;
    let inline_message_id = msg.inline_message_id.unwrap_or_default();
    log::debug!("inline message id = {}", inline_message_id.to_owned());
    match retry_future!(bot.edit_message_text_inline(inline_message_id.to_owned(), format!("{}\n\n_Asking question\\.\\.\\._", query.to_owned()))
        .parse_mode(ParseMode::MarkdownV2)
    ) {
        Ok(_) => {
            match retry_future!(api.single_message_dialog(query.to_owned(), MAX_TOKENS)){
                Ok(reply) => {
                    log::debug!("received response from DeepSeek = {}", escape_markdown(reply.to_owned()));
                    match retry_future!(bot.edit_message_text_inline(inline_message_id.to_owned(), format!("*Q: {}*\nA: {}", query, escape_markdown(reply.to_owned())))
                        .parse_mode(ParseMode::MarkdownV2)
                    ) {
                        Ok(_) => {
                            log::debug!("sent response = {}", escape_markdown(reply));
                            match retry_future!(bot.edit_message_reply_markup_inline(inline_message_id.to_owned())
                                .reply_markup(generate_keyboard())) {
                                Ok(_) => (),
                                Err(e) => log::error!("Error updating inline button: {}", e),
                            }
                        }
                        Err(e) => log::error!("Error sending response: {}", e)
                    }
                }
                Err(e) => log::error!("Unable to get response from DeepSeek: {}", e)
            }
        }
        Err(e) => log::error!("Error when updating inline hint: {}", e)
    }
    Ok(())
}

async fn chat_handler(bot: Bot, msg: Message, api: DeepSeekAPI) -> ResponseResult<()> {
    log::debug!("called chat_handler");
    if msg.via_bot != None {
        return Ok(())
    }
    match msg.text() {
        Some(text) => {
            log::debug!("Received msg = {}", text);
            let mut response = String::from("You are seeing this message because there was an error when we communicate with DeepSeek. Check the log for details.");
            match retry_future!(api.single_message_dialog(String::from(text), MAX_TOKENS)) {
                Ok(reply) => {
                    log::debug!("received response from DeepSeek = {}", escape_markdown(reply.to_owned()));
                    response = reply;
                },
                Err(e) => {
                    log::error!("Unable to get response from DeepSeek: {}", e);
                },
            }
            match retry_future!(reply_to_message(bot.to_owned(), msg.to_owned(), response.to_owned())) {
                Ok(msg) => log::debug!("sent response = {}", msg),
                Err(e) => log::error!("Error sending response: {}", e),
            }
        },
        None => (),
    };

    Ok(())
}

async fn command_handler(bot: Bot, msg: Message, cmd: Command, api: DeepSeekAPI) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            retry_future!(reply_to_message(bot.to_owned(), msg.to_owned(), Command::descriptions().to_string()))?;
        }
        Command::Die => {
            retry_future!(bot.send_dice(msg.chat.id))?;
        }
        Command::Info => {
            match retry_future!(api.get_balance()) {
                Ok(reply) => {
                    retry_future!(reply_to_message(bot.to_owned(), msg.to_owned(), reply.to_owned()))?;
                }
                Err(e) => {
                    log::error!("Error when fetching balance information: {}", e);
                }
            }
        }
    };

    Ok(())
}

async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    let config = toml::from_str::<Config>(std::fs::read_to_string("config.toml")?.as_str())?;
    log::info!("Read bot token = {}", config.telegram_bot_token);

    let bot = Bot::new(config.telegram_bot_token);
    let deepseek_api_token = config.deepseek_api_token;
    let deepseek_api_token1 = deepseek_api_token.clone();
    let deepseek_api_token2 = deepseek_api_token.clone();
    let deepseek_api_token3 = deepseek_api_token.clone();
    Dispatcher::builder(bot, dptree::entry()
        .branch(
            Update::filter_chosen_inline_result().endpoint(move |bot: Bot, msg: ChosenInlineResult | {
                let deepseek_api_token = deepseek_api_token3.to_owned();
                async move {
                    inline_result_handler(bot, msg, DeepSeekAPI { token: deepseek_api_token, timeout: TIMEOUT }).await
                }
            })
        ).branch(
            Update::filter_inline_query().endpoint(inline_handler),
        ).branch(
            Update::filter_message().filter_command::<Command>().endpoint(move |bot: Bot, msg: Message, cmd: Command| {
                let deepseek_api_token = deepseek_api_token1.to_owned();
                async move {
                    command_handler(bot, msg, cmd, DeepSeekAPI { token: deepseek_api_token, timeout: TIMEOUT }).await
                }
            }),
        ).branch(
            Update::filter_message().endpoint(move |bot: Bot, msg: Message| {
                let deepseek_api_token = deepseek_api_token2.to_owned();
                async move {
                    chat_handler(bot, msg, DeepSeekAPI { token: deepseek_api_token, timeout: TIMEOUT }).await
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
