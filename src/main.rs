use teloxide::types::*;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

#[derive(serde::Deserialize)]
struct Config {
    telegram_bot_token: String,
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "display this text")]
    Help,
    #[command(description = "test connectivity")]
    Die,
}

fn reply_to_message_id(message_id: MessageId) -> ReplyParameters {
    ReplyParameters {
        message_id,
        chat_id: None,
        allow_sending_without_reply: Some(false),
        quote: None,
        quote_parse_mode: None,
        quote_entities: None,
        quote_position: None,
    }
}

async fn chat_handler(bot: Bot, msg: Message) -> ResponseResult<()> {
    log::debug!("called chat_handler");
    match msg.text() {
        Some(text) => {
            bot.send_message(msg.chat.id, "Just testing. This message should look like a reply to the original message.")
                .message_thread_id(msg.thread_id.unwrap())
                .reply_parameters(reply_to_message_id(msg.id))
                .await?;
            log::debug!("Received msg = {}", text);
        },
        None => (),
    };

    Ok(())
}

async fn command_handler(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Help => bot.send_message(msg.chat.id, Command::descriptions().to_string())
            .message_thread_id(msg.thread_id.unwrap())
            .await?,
        Command::Die => bot.send_dice(msg.chat.id)
            .message_thread_id(msg.thread_id.unwrap())
            .await?,
    };

    Ok(())
}

async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    let config = toml::from_str::<Config>(std::fs::read_to_string("config.toml")?.as_str())?;
    log::info!("Read bot token = {}", config.telegram_bot_token);

    let bot = Bot::new(config.telegram_bot_token);
    Dispatcher::builder(bot, dptree::entry()
        .branch(
            Update::filter_message().filter_command::<Command>().endpoint(command_handler),
        ).branch(
            Update::filter_message().endpoint(chat_handler),
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
