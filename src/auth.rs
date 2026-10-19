use std::{env, future::Future, io, pin::Pin};

use actix_web::{
    FromRequest, HttpRequest,
    cookie::{Cookie, SameSite, time::Duration},
    dev::Payload,
    http::header::AUTHORIZATION,
    web::Data,
};
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode, get_current_timestamp,
};
use redis::{AsyncCommands, aio::ConnectionManager};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::AppError;

pub const TOKEN_TTL_SECONDS: u64 = 3_600;
const JWT_COOKIE: &str = "andromeda_access_token";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub iat: u64,
    pub exp: u64,
    pub jti: String,
}

pub struct JwtAuth {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    blacklist: ConnectionManager,
    secure_cookie: bool,
}

impl JwtAuth {
    pub async fn from_env() -> io::Result<Self> {
        let secret = env::var("JWT_SECRET")
            .map_err(|_| io::Error::other("Задайте JWT_SECRET: не менее 32 байт"))?;
        if secret.len() < 32 {
            return Err(io::Error::other(
                "JWT_SECRET должен содержать не менее 32 байт",
            ));
        }
        let redis_url =
            env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_owned());
        let client = redis::Client::open(redis_url)
            .map_err(|error| io::Error::other(format!("Ошибка адреса Redis: {error}")))?;
        let blacklist = client
            .get_connection_manager()
            .await
            .map_err(|error| io::Error::other(format!("Ошибка подключения к Redis: {error}")))?;
        Ok(Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            blacklist,
            secure_cookie: env::var("COOKIE_SECURE").is_ok_and(|value| value == "true"),
        })
    }

    pub fn issue(&self, user_id: i64, username: &str) -> Result<String, AppError> {
        issue_token(&self.encoding_key, user_id, username)
    }

    async fn authenticate(&self, token: &str) -> Result<AuthenticatedUser, AppError> {
        let claims = decode_claims(token, &self.decoding_key)?;
        let mut redis = self.blacklist.clone();
        let revoked: bool = redis
            .exists(blacklist_key(&claims.jti))
            .await
            .map_err(|error| AppError::Auth(error.to_string()))?;
        if revoked {
            return Err(AppError::Unauthorized);
        }
        let id = claims.sub.parse().map_err(|_| AppError::Unauthorized)?;
        Ok(AuthenticatedUser { id, claims })
    }

    pub async fn revoke(&self, claims: &Claims) -> Result<(), AppError> {
        let ttl = claims.exp.saturating_sub(get_current_timestamp());
        if ttl > 0 {
            let mut redis = self.blacklist.clone();
            let _: () = redis
                .set_ex(blacklist_key(&claims.jti), &claims.sub, ttl)
                .await
                .map_err(|error| AppError::Auth(error.to_string()))?;
        }
        Ok(())
    }

    // The Tera forms carry the same JWT in an HttpOnly cookie. API routes accept Bearer only.
    pub fn browser_cookie(&self, token: String) -> Cookie<'static> {
        Cookie::build(JWT_COOKIE, token)
            .path("/")
            .http_only(true)
            .same_site(SameSite::Lax)
            .secure(self.secure_cookie)
            .max_age(Duration::seconds(TOKEN_TTL_SECONDS as i64))
            .finish()
    }

    pub fn removal_cookie(&self) -> Cookie<'static> {
        let mut cookie = self.browser_cookie(String::new());
        cookie.make_removal();
        cookie
    }
}

fn blacklist_key(jti: &str) -> String {
    format!("jwt:blacklist:{jti}")
}

fn issue_token(key: &EncodingKey, user_id: i64, username: &str) -> Result<String, AppError> {
    let now = get_current_timestamp();
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_owned(),
        iat: now,
        exp: now + TOKEN_TTL_SECONDS,
        jti: Uuid::new_v4().to_string(),
    };
    encode(&Header::new(Algorithm::HS256), &claims, key)
        .map_err(|error| AppError::Auth(error.to_string()))
}

fn decode_claims(token: &str, key: &DecodingKey) -> Result<Claims, AppError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.leeway = 0;
    validation.validate_nbf = true;
    validation.set_required_spec_claims(&["sub", "exp"]);
    let claims = decode::<Claims>(token, key, &validation)
        .map_err(|_| AppError::Unauthorized)?
        .claims;
    let now = get_current_timestamp();
    if claims.exp <= now
        || claims.iat > now
        || claims.exp <= claims.iat
        || claims.sub.parse::<i64>().map_or(true, |id| id <= 0)
        || claims.username.is_empty()
        || Uuid::parse_str(&claims.jti).is_err()
    {
        return Err(AppError::Unauthorized);
    }
    Ok(claims)
}

fn request_token(request: &HttpRequest) -> Result<String, AppError> {
    if let Some(header) = request.headers().get(AUTHORIZATION) {
        let mut parts = header
            .to_str()
            .map_err(|_| AppError::Unauthorized)?
            .split_whitespace();
        let scheme = parts.next().ok_or(AppError::Unauthorized)?;
        let token = parts.next().ok_or(AppError::Unauthorized)?;
        if !scheme.eq_ignore_ascii_case("Bearer") || parts.next().is_some() {
            return Err(AppError::Unauthorized);
        }
        return Ok(token.to_owned());
    }
    if !request.path().starts_with("/api/")
        && let Some(cookie) = request.cookie(JWT_COOKIE)
    {
        return Ok(cookie.value().to_owned());
    }
    Err(AppError::Unauthorized)
}

async fn authenticate_request(request: &HttpRequest) -> Result<AuthenticatedUser, AppError> {
    let token = request_token(request)?;
    let auth = request
        .app_data::<Data<JwtAuth>>()
        .ok_or_else(|| AppError::Auth("JWT не настроен".to_owned()))?;
    auth.authenticate(&token).await
}

// Public reads remain available with absent, invalid, expired or revoked credentials.
// If authentication infrastructure is unavailable, public reads use guest flags (0/1).
pub async fn optional_user(request: &HttpRequest) -> Option<AuthenticatedUser> {
    authenticate_request(request).await.ok()
}

pub async fn optional_user_id(request: &HttpRequest) -> Option<i64> {
    optional_user(request).await.map(|user| user.id)
}

pub struct AuthenticatedUser {
    pub id: i64,
    pub claims: Claims,
}

impl FromRequest for AuthenticatedUser {
    type Error = AppError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(request: &HttpRequest, _: &mut Payload) -> Self::Future {
        let request = request.clone();
        Box::pin(async move { authenticate_request(&request).await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;

    const SECRET: &[u8] = b"jwt-test-secret-at-least-32-bytes-long";

    #[test]
    fn issued_token_has_user_expiry_and_unique_id() {
        let key = EncodingKey::from_secret(SECRET);
        let first = issue_token(&key, 42, "student").unwrap();
        let second = issue_token(&key, 42, "student").unwrap();
        let claims = decode_claims(&first, &DecodingKey::from_secret(SECRET)).unwrap();
        assert_eq!(claims.sub, "42");
        assert_eq!(claims.username, "student");
        assert_eq!(claims.exp - claims.iat, TOKEN_TTL_SECONDS);
        assert_ne!(first, second);
    }

    #[test]
    fn invalid_signature_and_algorithm_are_rejected() {
        let token = issue_token(&EncodingKey::from_secret(SECRET), 42, "student").unwrap();
        assert!(decode_claims(&token, &DecodingKey::from_secret(b"another-secret")).is_err());
        let claims = decode_claims(&token, &DecodingKey::from_secret(SECRET)).unwrap();
        let wrong_algorithm = encode(
            &Header::new(Algorithm::HS384),
            &claims,
            &EncodingKey::from_secret(SECRET),
        )
        .unwrap();
        assert!(decode_claims(&wrong_algorithm, &DecodingKey::from_secret(SECRET)).is_err());
    }

    #[test]
    fn expired_and_invalid_user_tokens_are_rejected() {
        let now = get_current_timestamp();
        let mut claims = Claims {
            sub: "42".to_owned(),
            username: "student".to_owned(),
            iat: now - 100,
            exp: now - 1,
            jti: Uuid::new_v4().to_string(),
        };
        let key = EncodingKey::from_secret(SECRET);
        let expired = encode(&Header::default(), &claims, &key).unwrap();
        assert!(decode_claims(&expired, &DecodingKey::from_secret(SECRET)).is_err());
        claims.exp = now + 100;
        claims.sub = "0".to_owned();
        let invalid_user = encode(&Header::default(), &claims, &key).unwrap();
        assert!(decode_claims(&invalid_user, &DecodingKey::from_secret(SECRET)).is_err());
    }

    #[test]
    fn api_requires_bearer_even_with_browser_cookie() {
        let request = TestRequest::with_uri("/api/andromeda-stars/draft")
            .cookie(Cookie::new(JWT_COOKIE, "browser-token"))
            .to_http_request();
        assert!(request_token(&request).is_err());
        let browser_request = TestRequest::with_uri("/andromeda-stars/draft")
            .cookie(Cookie::new(JWT_COOKIE, "browser-token"))
            .to_http_request();
        assert_eq!(request_token(&browser_request).unwrap(), "browser-token");
        let bearer_request = TestRequest::with_uri("/api/andromeda-stars/draft")
            .insert_header((AUTHORIZATION, "Bearer api-token"))
            .to_http_request();
        assert_eq!(request_token(&bearer_request).unwrap(), "api-token");
    }
}
