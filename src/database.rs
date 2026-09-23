use std::{env, io};

use sea_orm::{ColumnTrait, Database, DatabaseConnection, EntityTrait, QueryFilter};
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::{
    errors::AppError,
    models::andromeda_users::{Column as UserColumn, Entity as Users},
};

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

pub async fn current_andromeda_user_id(database: &AppDatabase) -> Result<i64, AppError> {
    let username =
        env::var("ANDROMEDA_CURRENT_USERNAME").unwrap_or_else(|_| "andromeda_student".to_owned());

    Users::find()
        .filter(UserColumn::Username.eq(&username))
        .one(&database.orm)
        .await?
        .map(|user| user.user_id)
        .ok_or_else(|| AppError::NotFound(format!("пользователь Андромеды с именем {}", username)))
}
