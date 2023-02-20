use teloxide::{
    prelude::*,
    types::{InlineQueryResult, InlineQueryResultArticle, InputMessageContent, InputMessageContentText},
    utils::command::BotCommands,
};
use crate::db;
use crate::{Command, Config};
use crate::tg;

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
        bot.send_message(msg.chat.id, "⚠️").await?;
        return Ok(());
    } 
    let mut user = user.unwrap();

    match cmd {
        Command::Start => { bot.send_message(msg.chat.id, "😏").await?; }
        Command::Help => { bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?; }
        Command::LoginInfo { username, pwd } => {
            user.username = username;
            user.pwd = pwd;

            let sync_res = db::sync_user(&cfg.conn, &user).await;
            let text = match sync_res {
                Ok(()) => "👌",
                Err(()) => "⚠️",
            };
            bot.send_message(msg.chat.id, text).await?;
        }
        Command::SetSemester { semester } => {
            if !semester > 0 && !semester < 9 {
                bot.send_message(msg.chat.id, "Семестр, епта, от 1 до 8, если кто не знал").await?;
                return Ok(());
            }

            user.semester = semester as u8;
            let sync_res = db::sync_user(&cfg.conn, &user).await;

            if sync_res.is_ok() {
                let del_res = db::delete_rating(&cfg.conn, &user).await;
                if del_res.is_ok() {
                    bot.send_message(msg.chat.id, "👌, Теперь придется подождать, пока твой рейтинг обновится, я пришлю уведомление (это займет не больше 20 минут))").await?;
                    return Ok(());
                }
            }
            bot.send_message(msg.chat.id, "⚠️").await?;
        }
        Command::GetRating => {
            if user.username.is_empty() || user.pwd.is_empty() || user.semester == 0 {
                bot.send_message(msg.chat.id, "Надо ввести логин, пароль и семестр").await?;
                return Ok(());
            } 

            let rating = db::get_rating(&cfg.conn, &user).await;
            if rating.is_none() {
                bot.send_message(msg.chat.id, "У тебя пустой рейтинг").await?;
                return Ok(());
            } 

            let text = rating
            .unwrap()
            .iter()
            .map(|subject| subject.to_string())
            .collect::<Vec<String>>()
            .join("\n\n");

            bot.send_message(msg.chat.id, text).await?;    
        }
        Command::Stats => {
            if msg.chat.id != cfg.bot_maintainer.into() {
                bot.send_message(msg.chat.id, "😑").await?;
                return Ok(());
            }

            let users = db::get_users(&cfg.conn).await;
            if users.is_none() {
                bot.send_message(msg.chat.id, "⚠️").await?;
                return Ok(());
            }
            
            let mut handles= Vec::new();
            for user in users.unwrap() {
                handles.push(tokio::spawn(tg::get_user_string(bot.clone(), user)));
            }
            
            let mut results = Vec::with_capacity(handles.len());
            for handle in handles {
                let res = handle.await;
                match res {
                    Ok(string) => { results.push(string); }
                    Err(_err) => ()
                }
            }

            bot.send_message(msg.chat.id, results.join("\n")).await?;
        }
    };

    Ok(())
}