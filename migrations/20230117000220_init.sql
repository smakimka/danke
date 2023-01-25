CREATE TABLE IF NOT EXISTS users 
(
    id integer primary key, 
    chat_id INTEGER, 
    username TEXT, 
    pwd TEXT,
    semester INTEGER
);
CREATE TABLE IF NOT EXISTS rating 
(
    id integer primary key, 
    user_id INTEGER, 
    subject_name TEXT,
    attendance REAL,
    control REAL,
    creative REAL,
    test REAL
);