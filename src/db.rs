use log::{info , warn};

#[derive(sqlx::FromRow, Debug)]
pub(crate) struct User {
    id: i64,
    chat_id: i64,
    pub(crate) username: String,
    pub(crate) pwd: String
}

pub(crate) async fn get_user(conn: &sqlx::Pool<sqlx::Sqlite>, user_chat_id: i64) -> Option<User> {
    let user = sqlx::query_as::<_, User>("SELECT id, chat_id, username, pwd FROM users where chat_id = ?")
    .bind(user_chat_id)
    .fetch_one(conn)
    .await;

    if user.is_err() {
        info!("User with id {} not found, creating new", user_chat_id);
        let user_id = sqlx::query!("INSERT into users (chat_id) values (?)", user_chat_id)
        .execute(conn)
        .await;

        if user_id.is_err() { return None; }
        
        let user = User { 
            id: user_id.unwrap().last_insert_rowid(), 
            chat_id: user_chat_id,
            username: "".to_string(), 
            pwd: "".to_string()
        };
        return Some(user);
    }

    Some(user.unwrap())
}

pub(crate) async fn sync_user(conn: &sqlx::Pool<sqlx::Sqlite>, user: &User) -> Result<(), ()> {
    let query_res = sqlx::query!("UPDATE users set username = ?, pwd = ? where id = ?", user.username, user.pwd, user.id)
    .execute(conn).await;

    if query_res.is_err() { return Err(()); }
    Ok(())
}