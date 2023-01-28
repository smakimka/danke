use std::collections::HashMap;
use crate::rating;

#[derive(sqlx::FromRow, Debug, Clone)]
pub(crate) struct User {
    pub(crate) id: i64,
    pub(crate) chat_id: i64,
    pub(crate) username: String,
    pub(crate) pwd: String,
    pub(crate) semester: u8
}

pub(crate) async fn get_user(conn: &sqlx::Pool<sqlx::Sqlite>, user_chat_id: i64) -> Option<User> {
    let user = sqlx::query_as::<_, User>("SELECT id, chat_id, username, pwd, semester FROM users where chat_id = ?")
    .bind(user_chat_id)
    .fetch_one(conn)
    .await;

    if user.is_err() {
        log::info!("User with id {} not found, creating new", user_chat_id);
        let user_id = sqlx::query!("INSERT into users (chat_id) values (?)", user_chat_id)
            .execute(conn)
            .await;

        if user_id.is_err() { return None; }
        
        let user = User { 
            id: user_id.unwrap().last_insert_rowid(), 
            chat_id: user_chat_id,
            username: "".to_string(), 
            pwd: "".to_string(),
            semester: 0
        };
        return Some(user);
    }

    Some(user.unwrap())
}

pub(crate) async fn sync_user(conn: &sqlx::Pool<sqlx::Sqlite>, user: &User) -> Result<(), ()> {
    let query_res = sqlx::query!("UPDATE users set username = ?, pwd = ?, semester = ? where id = ?", user.username, user.pwd, user.semester, user.id)
    .execute(conn).await;

    if let Err(err) = query_res {
        log::error!("{}", err.to_string());
        return Err(());
    }
    Ok(())
}

pub(crate) async fn get_rating(conn: &sqlx::Pool<sqlx::Sqlite>, user: &User) -> Option<Vec<rating::Subject>> {
    let user_rating = sqlx::query_as::<_, rating::Subject>("SELECT subject_name as name, attendance, control, creative, test FROM rating where user_id = ?")
        .bind(user.id)
        .fetch_all(conn)
        .await;

    if let Err(err) = user_rating {
        log::error!("{}", err.to_string());
        return None;
    }

    let user_rating = user_rating.unwrap();
    if user_rating.is_empty() {
        return None;
    }

    Some(user_rating)
}

pub(crate) async fn get_rating_map(conn: &sqlx::Pool<sqlx::Sqlite>, user: &User) -> Option<HashMap<String, rating::Subject>> {
    let user_rating = sqlx::query_as::<_, rating::Subject>("SELECT subject_name as name, attendance, control, creative, test FROM rating where user_id = ?")
        .bind(user.id)
        .fetch_all(conn)
        .await;
    if user_rating.is_err() { return None; }

    let mut map: HashMap<String, rating::Subject> = HashMap::new(); 
    for subject in user_rating.unwrap() {
        map.insert(subject.name.to_owned(), subject);
    }

    Some(map)
}

pub(crate) async fn delete_rating(conn: &sqlx::Pool<sqlx::Sqlite>, user: &User) -> Result<(), ()> {
    let query_res = sqlx::query!("delete from rating where user_id = ?", user.id)
    .execute(conn).await;

    if query_res.is_err() { return Err(()); }
    Ok(())
}