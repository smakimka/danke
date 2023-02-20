use teloxide::requests::Requester;

use crate::db::User;
use teloxide::types::ChatId;

pub(crate) async fn get_user_string(bot: teloxide::Bot, user: User) -> String {
    let chat = bot.get_chat( ChatId { 0: user.chat_id }).await;

    if chat.is_ok() {
        let chat = chat.unwrap();
        let username = chat.username();
        if username.is_some() {
            return format!("User {} [login: {}, sem: {}]\n", user.id, username.unwrap(), user.semester);
        }

        let first_name = chat.first_name();
        let last_name = chat.last_name();

        if first_name.is_some() && last_name.is_some() {
            return format!("User {} [login: {} {}, sem: {}]\n", user.id, first_name.unwrap(), last_name.unwrap(), user.semester);
        }
    }

    format!("User {} [login: {}, sem: {}]\n", user.id, user.username, user.semester)
}