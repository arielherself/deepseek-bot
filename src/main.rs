mod user;
mod config;
use deepseek::api::DeepSeekAPI;
use deepseek::search;
use reqwest::Client;
use teloxide::payloads::EditMessageTextInlineSetters;
use teloxide::payloads::SendMessageSetters;
use teloxide::RequestError;
use teloxide::types::*;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

const MAX_RETRY: usize = 10;
const TIMEOUT: u64 = 1000 * 60 * 10;
const MAX_TOKEN: u64 = 512;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "display this text")]
    Help,
    #[command(description = "test connectivity")]
    Die,
    #[command(description = "get account information")]
    Info,
    #[command(description = "allow one user to query")]
    Grant,
}

macro_rules! retry_future {
    ($future:expr) => {{
        let mut result = $future.await;
        if matches!(result, Err(_)) {
            for i in 1..MAX_RETRY {
                log::debug!("Retrying: {}/{MAX_RETRY}", i + 1);
                let new_result = $future.await;
                if matches!(new_result, Ok(_)) {
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
    // truncate the string to meet Telegram API requirement
    let text = String::from_utf8_lossy(text.as_bytes().into_iter().cloned().take(2048).collect::<Vec<u8>>().as_slice()).to_string();
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

fn check_user_valid(user: User) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    match user::check_uid(user.id.0.to_string())? {
        user::Role::Untrusted => Ok(false),
        _ => Ok(true),
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
    let _ = api.get_balance().await;
    let query = msg.query;
    let inline_message_id = msg.inline_message_id.unwrap_or_default();
    log::debug!("inline message id = {}", inline_message_id.to_owned());
    match retry_future!(bot.edit_message_text_inline(inline_message_id.to_owned(), format!("{}\n\n_Asking question\\.\\.\\._", escape_markdown(query.to_owned())))
        .parse_mode(ParseMode::MarkdownV2)
    ) {
        Ok(_) => {
            let role = check_user_valid(msg.from.to_owned());
            match role {
                Ok(valid) => {
                    if !valid {
                        match retry_future!(bot.edit_message_text_inline(inline_message_id.to_owned(), format!("*Q: {}*\n\n_User does not have permission\\._", escape_markdown(query.to_owned())))
                            .parse_mode(ParseMode::MarkdownV2)
                        ) {
                            Ok(_) => (),
                            Err(e) => log::error!("Error updating inline hint: {}", e),
                        }
                        return Ok(());
                    }
                }
                Err(e) => {
                    log::error!("Error when checking role: {}", e);
                    return Ok(());
                }
            }
            let search_driver = search::SearchDriver::from(api.to_owned());
            let need_search = retry_future!(search_driver.determine(query.to_owned()));
            let system_prompt = match need_search {
                Ok(need_search) => {
                    if need_search {
                        log::debug!("Search invoked.");
                        let system_prompt = retry_future!(search_driver.search_and_summary(query.to_owned()));
                        match system_prompt {
                            Ok(system_prompt) => system_prompt,
                            Err(e) => {
                                log::error!("Error when fetching system prompt: {}", e);
                                String::new()
                            }
                        }
                    } else {
                        String::new()
                    }
                },
                Err(e) => {
                    log::error!("Error when determining need_search: {}", e);
                    String::new()
                }
            };
            match retry_future!(api.single_message_dialog_with_system(MAX_TOKEN, query.to_owned(), system_prompt.to_owned())) {
                Ok(reply) => {
                    log::debug!("received response from DeepSeek = {}", escape_markdown(reply.to_owned()));
                    match retry_future!(bot.edit_message_text_inline(inline_message_id.to_owned(), format!("*Q: {}*\nA: {}\n{}", escape_markdown(query.to_owned()), escape_markdown(reply.to_owned()), if system_prompt.len() > 0 { String::from("> Searching invoked\\. The answer may contain information from the Internet\\.") } else { String::new() }))
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
    if let Some(user) = msg.to_owned().from {
        match check_user_valid(user) {
            Ok(valid) => {
                if !valid {
                    match retry_future!(reply_to_message(bot.to_owned(), msg.to_owned(), String::from("User doesn't have permission."))) {
                        Ok(_) => (),
                        Err(e) => log::error!("Error sending permission information: {}", e),
                    }
                    return Ok(());
                }
            }
            Err(e) => {
                log::error!("Error checking user permission: {}", e);
                return Ok(());
            }
        }
    } else {
        return Ok(())
    }
    match msg.text() {
        Some(text) => {
            log::debug!("Received msg = {}", text);
            let mut response = String::from("You are seeing this message because there was an error when we communicate with DeepSeek. Check the log for details.");
            match retry_future!(api.single_message_dialog(MAX_TOKEN, String::from(text))) {
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
        Command::Grant => {
            if let Some(user) = msg.to_owned().from {
                match user::check_uid(user.id.0.to_string()) {
                    Ok(role) => {
                        if matches!(role, user::Role::SuperUser) {
                            match msg.to_owned().reply_to_message() {
                                Some(replied) => {
                                    if let Some(user) = replied.to_owned().from {
                                        match config::add_trusted_user(user.id.0.to_string()) {
                                            Ok(()) => match retry_future!(
                                                reply_to_message(bot.to_owned(), msg.to_owned(), String::from("Successfully granted permission."))
                                            ) {
                                                Ok(_) => (),
                                                Err(e) => log::error!("Cannot send message: {}", e),
                                            },
                                            Err(e) => log::error!("Cannot grant permission: {}", e),
                                        }
                                    }
                                }
                                None => {
                                    match retry_future!(
                                        reply_to_message(bot.to_owned(), msg.to_owned(), String::from("Please reply a message that's sent by another user."))
                                    ) {
                                        Ok(_) => (),
                                        Err(e) => log::error!("Cannot send message: {}", e),
                                    }
                                }
                            }
                        } else {
                            match retry_future!(
                                reply_to_message(bot.to_owned(), msg.to_owned(), String::from("You are not a superuser."))
                            ) {
                                Ok(_) => (),
                                Err(e) => log::error!("Cannot send message: {}", e),
                            }
                        }
                    }
                    Err(e) => log::error!("Error when checking user permission: {}", e)
                }
            }
        }
    };

    Ok(())
}

async fn serve() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = config::get_config()?;
    log::info!("Read bot token = {}", config.telegram_bot_token);

    let client = Client::new();

    let bot = Bot::new(config.telegram_bot_token);
    let deepseek_api_token = config.deepseek_api_token;
    Dispatcher::builder(bot, dptree::entry()
        .branch(
            {
                let deepseek_api_token = deepseek_api_token.clone();
                let client = client.clone();
                Update::filter_chosen_inline_result().endpoint(move |bot: Bot, msg: ChosenInlineResult | {
                    let deepseek_api_token = deepseek_api_token.clone();
                    let client = client.clone();
                    async move {
                        inline_result_handler(bot, msg, DeepSeekAPI { token: deepseek_api_token, timeout: TIMEOUT, client }).await
                    }
                })
            }
        ).branch(
            Update::filter_inline_query().endpoint(inline_handler),
        ).branch(
            {
                let deepseek_api_token = deepseek_api_token.clone();
                let client = client.clone();
                Update::filter_message().filter_command::<Command>().endpoint(move |bot: Bot, msg: Message, cmd: Command| {
                    let deepseek_api_token = deepseek_api_token.clone();
                    let client = client.clone();
                    async move {
                        command_handler(bot, msg, cmd, DeepSeekAPI { token: deepseek_api_token, timeout: TIMEOUT, client: client }).await
                    }
                })
            }
        ).branch(
        {
                let deepseek_api_token = deepseek_api_token.clone();
                let client = client.clone();
                Update::filter_message().endpoint(move |bot: Bot, msg: Message| {
                    let deepseek_api_token = deepseek_api_token.clone();
                    let client = client.clone();
                    async move {
                        chat_handler(bot, msg, DeepSeekAPI { token: deepseek_api_token, timeout: TIMEOUT, client: client }).await
                    }
                })
            }
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
