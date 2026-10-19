use std::{env, io};

use sea_orm::{Database, DatabaseConnection};
use sqlx::{PgPool, postgres::PgPoolOptions};

#[derive(Clone)]
pub struct AppDatabase {
    pub orm: DatabaseConnection,
    pub sql: PgPool,
}

pub async fn connect() -> io::Result<AppDatabase> {
    let database_url = env::var("DATABASE_URL")
        .map_err(|error| io::Error::other(format!("Не задан DATABASE_URL: {}", error)))?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .map_err(|error| io::Error::other(format!("Ошибка подключения к PostgreSQL: {}", error)))?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|error| io::Error::other(format!("Ошибка миграции PostgreSQL: {}", error)))?;

    let orm = Database::connect(&database_url)
        .await
        .map_err(|error| io::Error::other(format!("Ошибка подключения ORM: {}", error)))?;

    Ok(AppDatabase { orm, sql: pool })
}
