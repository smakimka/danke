use crate::db;
use crate::rating;
use std::collections::HashMap;
use crate::rating::Rating;

#[derive(Debug)]
struct Notification {
    pub(crate) chat_id: i64,
    pub(crate) message: String
}

async fn get_differences(conn: &sqlx::Pool<sqlx::Sqlite>) -> Option<Vec<Notification>> {
    let users = sqlx::query_as::<_, db::User>("SELECT * FROM users where not(pwd is null or pwd = '' or username is null or username = '' or semester is null or semester = 0)")
    .fetch_all(conn)
    .await;
    if users.is_err() { 
        log::error!("Couldn't get users");
        return None; 
    }
    let users = users.unwrap();

    let mut set: tokio::task::JoinSet<Option<Rating>> = tokio::task::JoinSet::new();  
    for user in users {
        set.spawn(rating::get_rating(user));
    }

    let mut new_ratings:Vec<Rating> = vec![];
    while let Some(Ok(Some(rating))) = set.join_next().await {
        new_ratings.push(rating);
    }

    if new_ratings.is_empty() {
        log::warn!("Couldn't get any new ratings");
        return None;
    }

    let mut notifications: Vec<Notification> = vec![];  
    for rating in new_ratings {
        let db_rating_map = db::get_rating_map(conn, &rating.user).await;
        if db_rating_map.is_none() {
            log::error!("Couldn't get rating map");
            return None;
        }
        let db_rating_map = db_rating_map.unwrap();
        
        if db_rating_map.is_empty() {
            for subject in rating.subjects {
                let rating_id = sqlx::query!("INSERT into rating (user_id, subject_name, attendance, control, creative, test) values (?, ?, ?, ?, ?, ?)", 
                    rating.user.id, subject.name, subject.attendance, subject.control, subject.creative, subject.test)
                .execute(conn)
                .await;

                if rating_id.is_err() {
                    log::error!("Couldn't insert into rating");
                    return None; 
                }
            }
            notifications.push(Notification { chat_id: rating.user.chat_id, message: teloxide::utils::markdown::escape("Рейтинг обновился (вероятно это уведомление из-за смены семестра)") });
        }
        else {
            let mut message: Vec<String> = vec![];
            for subject in rating.subjects {
                if db_rating_map.contains_key(&subject.name) {
                    let db_subject = db_rating_map.get(&subject.name).unwrap();
                    
                    let mut change: bool = false;
                    if subject.attendance != db_subject.attendance {
                        message.push(format!("||{}|| за посещение", teloxide::utils::markdown::escape(&(subject.attendance - db_subject.attendance).to_string())));
                        change = true;
                    }
                    if subject.creative != db_subject.creative {
                        message.push(format!("||{}|| по творческому", teloxide::utils::markdown::escape(&(subject.creative - db_subject.creative).to_string())));
                        change = true;
                    }
                    if subject.control != db_subject.control {
                        message.push(format!("||{}|| за контрольный", teloxide::utils::markdown::escape(&(subject.control - db_subject.control).to_string())));
                        change = true;
                    }
                    if subject.test != db_subject.test {
                        message.push(format!("||{}|| за экз/тест", teloxide::utils::markdown::escape(&(subject.test - db_subject.test).to_string())));
                        change = true;
                    }

                    if change {
                        let rating_id = sqlx::query!("update rating set attendance = ?, control = ?, creative = ?, test = ? where user_id = ? and subject_name = ?", 
                        subject.attendance, subject.control, subject.creative, subject.test, rating.user.id, subject.name)
                        .execute(conn)
                        .await;

                        if rating_id.is_err() {
                            log::error!("Couldn't update rating");
                            return None; 
                        }

                        message.push(format!("По {}\n", teloxide::utils::markdown::escape(&(subject.name).to_string())));
                    } 
                }
            }
            if !message.is_empty() {
                notifications.push(Notification { chat_id: rating.user.chat_id, message: message.join("\n") });
            }
        }
    };

    Some(notifications)
}

pub(crate) async fn run_updates(update_sleep_secs: u64, failed_update_sleep_secs: u64, db_url: &str) {
    let conn = sqlx::sqlite::SqlitePoolOptions::new()
    .max_connections(1)
    .connect(db_url)
    .await
    .unwrap();

    let client = reqwest::Client::new();
    let mut data = HashMap::with_capacity(3);
    data.insert("parse_mode", "MarkdownV2".to_string());

    let send_message_url = format!("https://api.telegram.org/bot{}/sendMessage", std::env::var("TELOXIDE_TOKEN").unwrap());

    loop {
        let notifications = get_differences(&conn).await;
        if notifications.is_none() { 
            log::warn!("Notifications returned with None"); 
            tokio::time::sleep(std::time::Duration::from_secs(failed_update_sleep_secs)).await;
            continue;
        }
        
        let mut set: tokio::task::JoinSet<Result<reqwest::Response, reqwest::Error>> = tokio::task::JoinSet::new();  
        for notification in notifications.unwrap() {
            let mut data = data.clone();
            data.insert("chat_id", notification.chat_id.to_string());
            data.insert("text", notification.message);

            set.spawn( client.post(&send_message_url)
                        .json(&data)
                        .send());
        }

        while let Some(res) = set.join_next().await {
            if res.is_err() {
                log::error!("error while sending notification: {}", res.err().unwrap().to_string())
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(update_sleep_secs)).await;
    }   
}