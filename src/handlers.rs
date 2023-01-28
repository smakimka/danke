use teloxide::{
    prelude::*,
    types::{InlineQueryResult, InlineQueryResultArticle, InputMessageContent, InputMessageContentText},
    utils::command::BotCommands,
};
use crate::db;
use crate::{Command, Config};

pub(crate) async fn inline_query_handler(
    bot: Bot,
    cfg: crate::Config,
    q: InlineQuery,
) -> Result<(), teloxide::RequestError> {
    let user = db::get_user(&cfg.conn, q.from.id.0 as i64).await;

    if user.is_none() {
        let answer = InlineQueryResultArticle::new(
            "1".to_string(),
            "There has been an error".to_string(),
            InputMessageContent::Text(InputMessageContentText::new("⚠️"))
        );
        let results = vec![InlineQueryResult::Article(answer)];
        let response = bot.answer_inline_query(&q.id, results).send().await;
        if let Err(err) = response {
            log::error!("Error in handler: {:?}", err);
        }
        return respond(());
    } 

    let user = user.unwrap();
    let rating = db::get_rating(&cfg.conn, &user).await;

    if rating.is_none() {
        let answer = InlineQueryResultArticle::new(
            "1".to_string(),
            "У тебя нету рейтинга".to_string(),
            InputMessageContent::Text(InputMessageContentText::new("⚠️"))
        );
        let results = vec![InlineQueryResult::Article(answer)];
        let response = bot.answer_inline_query(&q.id, results).send().await;
        if let Err(err) = response {
            log::error!("Error in handler: {:?}", err);
        }

        return respond(());
    }
    
    let rating = rating.unwrap();

    let mut results = vec![];
    for (subject_num, subject) in rating.into_iter().enumerate() {
        let desc = subject.to_string();

        let article = InlineQueryResultArticle::new(
            subject_num.to_string(),
            subject.name,
            InputMessageContent::Text(InputMessageContentText::new(desc))
        );
        results.push(InlineQueryResult::Article(article));
    }

    let response = bot.answer_inline_query(&q.id, results).is_personal(true).send().await;
    if let Err(err) = response {
        log::error!("Error in handler: {:?}", err);
    }

    respond(())
}

pub(crate) async fn commands_handler(
    bot: Bot,
    cfg: Config,
    msg: Message,
    cmd: Command,
) -> Result<(), teloxide::RequestError> {
    let user = db::get_user(&cfg.conn, msg.chat.id.0).await;
    if user.is_none() {
        return error_response(bot, msg).await;
    } 
    let mut user = user.unwrap();

    let text = match cmd {
        Command::Start => "😏".to_string(),
        Command::Help => { Command::descriptions().to_string() }
        Command::LoginInfo { username, pwd } => {
            user.username = username;
            user.pwd = pwd;

            let sync_res = db::sync_user(&cfg.conn, &user).await;
            if sync_res.is_err() {
                "⚠️".to_string()
            } else {
                "👌".to_string()
            }
        }
        Command::SetSemester { semester } => {
            if semester > 0 && semester < 9 {
                user.semester = semester as u8;
                let sync_res = db::sync_user(&cfg.conn, &user).await;
                if sync_res.is_err() {
                    "⚠️".to_string()
                } else {
                    let del_res = db::delete_rating(&cfg.conn, &user).await;
                    if del_res.is_err() {
                        "⚠️".to_string()
                    }
                    else {
                        "👌, Теперь придется подождать, пока твой рейтинг обновится, я пришлю уведомление (не больше 20 минут))".to_string()
                    }
                }
            }
            else {
                "Семестр, епта, от 1 до 8, если кто не знал".to_string()
            }
        }
        Command::GetRating => {
            if user.username.is_empty() || user.pwd.is_empty() || user.semester == 0 {
                "Надо ввести логин, пароль и семестр".to_string()
            } else {
                let rating = db::get_rating(&cfg.conn, &user).await;
                if rating.is_none() {
                    "У тебя пустой рейтинг".to_string()
                } else {
                    rating
                    .unwrap()
                    .iter()
                    .map(|subject| subject.to_string())
                    .collect::<Vec<String>>()
                    .join("\n\n")
                }
            }
        }
    };

    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}

async fn error_response(bot: Bot, msg: Message) -> Result<(), teloxide::RequestError> {
    bot.send_message(msg.chat.id, "⚠️").await?;
    Ok(())
}