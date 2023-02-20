use dotenv::dotenv;
use teloxide::{
    prelude::*,
    types::{Update, UserId},
    utils::command::BotCommands,
};

mod db;
mod handlers;
mod maintain;
mod rating;
mod tg;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Список досутпных команд:")]
enum Command {
    #[command(description = "Начать диалог")]
    Start,
    #[command(description = "Отобразить этот текст")]
    Help,
    #[command(
        description = "Установить логин и пароль (/logininfo login password)",
        parse_with = "split"
    )]
    LoginInfo { username: String, pwd: String },
    #[command(description = "Установить номер семестра (/setsemester 7)")]
    SetSemester { semester: i64 },
    #[command(description = "Получить рейтинг по всем предметам")]
    GetRating,
    #[command(description = "для одмина")]
    Stats,
}

#[derive(Clone)]
struct Config {
    bot_maintainer: UserId,
    conn: sqlx::Pool<sqlx::Sqlite>,
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let bot = Bot::from_env();
    let db_url = "sqlite:danke.db";

    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Warn)
        .init()
        .unwrap();

    let conn = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(10)
        .connect(db_url)
        .await
        .unwrap();

    let config = Config {
        bot_maintainer: UserId(434585640),
        conn,
    };

    let update_sleep_secs: u64 = 1200;
    let failed_update_sleep_secs: u64 = 600;

    tokio::spawn(async move {
        maintain::run_updates(update_sleep_secs, failed_update_sleep_secs, db_url).await
    });

    let inline_query_handler =
        Update::filter_inline_query().branch(dptree::endpoint(handlers::inline_query_handler));

    let message_handler = Update::filter_message()
        .branch(
            dptree::entry()
                .filter_command::<Command>()
                .endpoint(handlers::commands_handler),
        )
        .branch(dptree::endpoint(|msg: Message, bot: Bot| async move {
            bot.send_message(msg.chat.id, "😑").await?;
            respond(())
        }));

    let schema = dptree::entry()
        .branch(message_handler)
        .branch(inline_query_handler);

    Dispatcher::builder(bot, schema)
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
