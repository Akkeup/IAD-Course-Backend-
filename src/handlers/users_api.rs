use crate::{
    database::AppDatabase,
    errors::AppError,
    handlers::api_dto::{ApiAuthRequest, ApiRegisterRequest},
    models::andromeda_users::{self, Column as UserColumn, Entity as Users},
};
use actix_web::{
    HttpResponse, post,
    web::{Data, Json},
};
use argon2::{
    Argon2, PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

#[post("/users/register")]
pub async fn api_register_user(
    payload: Json<ApiRegisterRequest>,
    database: Data<AppDatabase>,
) -> Result<HttpResponse, AppError> {
    let payload = payload.validate()?;
    let username = payload.username;
    if Users::find()
        .filter(UserColumn::Username.eq(&username))
        .one(&database.orm)
        .await?
        .is_some()
    {
        return Err(AppError::Validation(
            "пользователь с таким username уже существует".to_owned(),
        ));
    }

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(payload.password.as_bytes(), &salt)
        .map_err(|error| AppError::Database(error.to_string()))?
        .to_string();
    andromeda_users::ActiveModel {
        username: Set(username),
        password_hash: Set(password_hash),
        ..Default::default()
    }
    .insert(&database.orm)
    .await?;

    Ok(HttpResponse::Created().finish())
}

#[post("/auth/login")]
pub async fn api_login(payload: Json<ApiAuthRequest>) -> Result<HttpResponse, AppError> {
    payload.validate()?;

    Ok(HttpResponse::NoContent().finish())
}

#[post("/auth/logout")]
pub async fn api_logout() -> Result<HttpResponse, AppError> {
    Ok(HttpResponse::NoContent().finish())
}
