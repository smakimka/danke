use std::sync::{Arc, Mutex};
use teloxide::{
    prelude::*,
    types::{Dice, Update, UserId},
    utils::command::BotCommands,
};
use dotenv::dotenv;
use log::{info , warn};

mod db;
mod lib;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "start the conversation")]
    Start,
    #[command(description = "display this text.")]
    Help,
    #[command(description = "handle a username and an age.", parse_with = "split")]
    LoginInfo { username: String, pwd: String },
    #[command(description = "get rating from rea website")]
    GetRating,
}

#[derive(Clone)]
struct Config {
    bot_maintainer: UserId,
    conn: sqlx::Pool<sqlx::Sqlite>
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let bot = Bot::from_env();

    simple_logger::SimpleLogger::new().with_level(log::LevelFilter::Warn).init().unwrap();

    let conn = sqlx::sqlite::SqlitePoolOptions::new()
    .max_connections(10)
    .connect("sqlite:danke.db")
    .await
    .unwrap();

    let config = Config {
        bot_maintainer: UserId(434585640),
        conn
    };

    let handler = Update::filter_message()
        .branch(
            dptree::entry()
                .filter_command::<Command>()
                .endpoint(commands_handler),
        )
        .branch(
            dptree::endpoint(|msg: Message, bot: Bot| async move {
                bot.send_message(msg.chat.id, "😑").await?;
                respond(())
            })
        );

    Dispatcher::builder(bot, handler)
        .default_handler(|upd| async move {
            log::warn!("Unhandled update: {:?}", upd);
        })
        .error_handler(LoggingErrorHandler::with_custom_text(
            "An error has occurred in the dispatcher",
        ))
        .dependencies(dptree::deps![config])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

async fn commands_handler(
    bot: Bot,
    cfg: Config,
    msg: Message,
    cmd: Command,
) -> Result<(), teloxide::RequestError> {
    let text: String;
    let user = db::get_user(&cfg.conn, msg.chat.id.0).await;
    if user.is_none() { text = "⚠️".to_string(); }
    else {
        let mut user = user.unwrap();
        text = match cmd {
            Command::Start => { 
                "😏".to_string()
            }
            Command::Help => { 
                Command::descriptions().to_string();
                format!("{:?}", user) 
            }
            Command::LoginInfo { username, pwd } => {
                user.username = username;
                user.pwd = pwd;

                let sync_res = db::sync_user(&cfg.conn, &user).await;
                if sync_res.is_err() { "⚠️".to_string() }
                else { "👌".to_string() }
            }
            Command::GetRating => {
                if user.username.is_empty() || user.pwd.is_empty() { "Надо ввести логин и пароль".to_string() }
                else { 
                    let rating = lib::get_rating(&user.username, &user.pwd, 7).await;
                    if rating.is_none() { "⚠️".to_string() }
                    else { 
                        rating.unwrap().iter().map(|subject| { subject.to_string() }).collect::<Vec<String>>().join("\n\n")
                    }
                }
            }
        };
    }

    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}