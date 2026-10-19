use crate::{
    auth::{AuthenticatedUser, JwtAuth, TOKEN_TTL_SECONDS, optional_user},
    database::AppDatabase,
    errors::AppError,
    handlers::api_dto::{ApiAuthRequest, ApiAuthResponse, ApiRegisterRequest, ApiUser},
    models::andromeda_users::{self, Column as UserColumn, Entity as Users},
};
use actix_web::{
    HttpRequest, HttpResponse, get,
    http::header::ContentType,
    post,
    web::{Data, Json},
};
use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

#[utoipa::path(post, path = "/api/users/register", tag = "Пользователи",
    request_body = ApiRegisterRequest,
    responses((status = 201, description = "Пользователь зарегистрирован; пароль сохранён как Argon2-хеш"),
              (status = 400, description = "Неверные данные или имя уже занято"))) ]
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

#[utoipa::path(post, path = "/api/auth/login", tag = "Пользователи",
    request_body = ApiAuthRequest,
    responses((status = 200, description = "Вход выполнен; access_token передавайте как Authorization: Bearer <JWT>", body = ApiAuthResponse),
              (status = 400, description = "Неверный формат данных"),
              (status = 401, description = "Неверный логин или пароль"))) ]
#[post("/auth/login")]
pub async fn api_login(
    payload: Json<ApiAuthRequest>,
    database: Data<AppDatabase>,
    auth: Data<JwtAuth>,
) -> Result<HttpResponse, AppError> {
    payload.validate()?;
    let user = Users::find()
        .filter(UserColumn::Username.eq(payload.username.trim()))
        .one(&database.orm)
        .await?
        .ok_or(AppError::Unauthorized)?;
    let password_hash =
        PasswordHash::new(&user.password_hash).map_err(|_| AppError::Unauthorized)?;
    Argon2::default()
        .verify_password(payload.password.as_bytes(), &password_hash)
        .map_err(|_| AppError::Unauthorized)?;

    let token = auth.issue(user.user_id, &user.username)?;
    Ok(HttpResponse::Ok()
        .cookie(auth.browser_cookie(token.clone()))
        .json(ApiAuthResponse {
            access_token: token,
            token_type: "Bearer".to_owned(),
            expires_in: TOKEN_TTL_SECONDS,
            user: ApiUser {
                id: user.user_id,
                username: user.username,
            },
        }))
}

#[utoipa::path(post, path = "/api/auth/logout", tag = "Пользователи",
    security(("bearerAuth" = [])),
    responses((status = 204, description = "JWT отозван: jti добавлен в blacklist Redis до exp"),
              (status = 401, description = "Требуется вход"))) ]
#[post("/auth/logout")]
pub async fn api_logout(
    user: AuthenticatedUser,
    auth: Data<JwtAuth>,
) -> Result<HttpResponse, AppError> {
    auth.revoke(&user.claims).await?;
    Ok(HttpResponse::NoContent()
        .cookie(auth.removal_cookie())
        .finish())
}

#[post("/auth/logout")]
pub async fn browser_logout(
    user: AuthenticatedUser,
    auth: Data<JwtAuth>,
) -> Result<HttpResponse, AppError> {
    auth.revoke(&user.claims).await?;
    Ok(HttpResponse::NoContent()
        .cookie(auth.removal_cookie())
        .finish())
}

#[get("/auth")]
pub async fn auth_page(
    request: HttpRequest,
    templates: Data<tera::Tera>,
) -> Result<HttpResponse, AppError> {
    let mut context = tera::Context::new();
    let username = optional_user(&request)
        .await
        .map(|user| user.claims.username);
    context.insert("username", &username);
    return Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(templates.render("auth.html", &context)?));
}
