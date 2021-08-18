use rusqlite::Connection;

pub fn make_db_conn() -> anyhow::Result<Connection> {
    let db_url = std::env::var("DATABASE").unwrap_or("docsbot.store".to_string());
    let conn = Connection::open(db_url).expect("failed to open database");

    Ok(conn)
}