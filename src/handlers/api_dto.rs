use actix_multipart::form::{MultipartForm, tempfile::TempFile, text::Text};
use serde::{Deserialize, Serialize};

use crate::errors::AppError;

const STAR_NAME_MAX_LENGTH: usize = 150;
const CATALOG_ID_MAX_LENGTH: usize = 150;
const DESCRIPTION_MAX_LENGTH: usize = 2_000;
const USERNAME_MAX_LENGTH: usize = 100;
const PASSWORD_MAX_LENGTH: usize = 128;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiStarsFilter {
    pub distance_kpc: Option<String>,
    pub page: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiFeedQuery {
    pub id: Option<i64>,
    pub next: Option<bool>,
}

#[derive(Debug, MultipartForm)]
#[multipart(deny_unknown_fields, duplicate_field = "deny")]
pub struct ApiCreateStarForm {
    #[multipart(limit = "1KiB")]
    pub name: Text<String>,
    #[multipart(limit = "5MiB")]
    pub image: TempFile,
    #[multipart(limit = "20MiB")]
    pub video: TempFile,
}

impl ApiCreateStarForm {
    pub fn validated_name(&self) -> Result<String, AppError> {
        validate_required_text("name", &self.name.0, STAR_NAME_MAX_LENGTH)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiPublishRequest {
    pub name: String,
    pub catalog_id: String,
    pub distance_kpc: f32,
    pub velocity_kms: i32,
    pub description: String,
}

pub struct ValidatedPublishRequest {
    pub name: String,
    pub catalog_id: String,
    pub distance_kpc: f32,
    pub velocity_kms: i32,
    pub description: String,
}

impl ApiPublishRequest {
    pub fn validate(&self) -> Result<ValidatedPublishRequest, AppError> {
        if !self.distance_kpc.is_finite() || !(0.0..=9_999.99).contains(&self.distance_kpc) {
            return Err(AppError::Validation(
                "distance_kpc должен быть числом от 0 до 9999.99".to_owned(),
            ));
        }
        if self.velocity_kms < 0 {
            return Err(AppError::Validation(
                "velocity_kms должен быть неотрицательным числом".to_owned(),
            ));
        }

        Ok(ValidatedPublishRequest {
            name: validate_required_text("name", &self.name, STAR_NAME_MAX_LENGTH)?,
            catalog_id: validate_required_text(
                "catalog_id",
                &self.catalog_id,
                CATALOG_ID_MAX_LENGTH,
            )?,
            distance_kpc: self.distance_kpc,
            velocity_kms: self.velocity_kms,
            description: validate_required_text(
                "description",
                &self.description,
                DESCRIPTION_MAX_LENGTH,
            )?,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiLikeRequest {
    pub value: i32,
}

impl ApiLikeRequest {
    pub fn validate(&self) -> Result<i32, AppError> {
        if self.value == 0 || self.value == 1 {
            Ok(self.value)
        } else {
            Err(AppError::Validation("value должен быть 0 или 1".to_owned()))
        }
    }
}

pub fn validate_resource_id(id: i64) -> Result<i64, AppError> {
    if id > 0 {
        Ok(id)
    } else {
        Err(AppError::Validation(
            "id должен быть положительным целым числом".to_owned(),
        ))
    }
}

pub fn page_offset(page: Option<u64>, page_size: u64) -> Result<(u64, u64), AppError> {
    let page = page.unwrap_or(1).max(1);
    let offset = page
        .saturating_sub(1)
        .checked_mul(page_size)
        .ok_or_else(|| AppError::Validation("слишком большое значение page".to_owned()))?;
    Ok((page, offset))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiRegisterRequest {
    pub username: String,
    pub password: String,
}

pub struct ValidatedRegisterRequest {
    pub username: String,
    pub password: String,
}

impl ApiRegisterRequest {
    pub fn validate(&self) -> Result<ValidatedRegisterRequest, AppError> {
        let username = validate_required_text("username", &self.username, USERNAME_MAX_LENGTH)?;
        if username.chars().count() < 3
            || !username.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '-')
            })
        {
            return Err(AppError::Validation(
                "username должен содержать от 3 до 100 латинских букв, цифр, _ или -".to_owned(),
            ));
        }
        validate_password(&self.password, 8)?;

        Ok(ValidatedRegisterRequest {
            username,
            password: self.password.clone(),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiAuthRequest {
    pub username: String,
    pub password: String,
}

impl ApiAuthRequest {
    pub fn validate(&self) -> Result<(), AppError> {
        validate_required_text("username", &self.username, USERNAME_MAX_LENGTH)?;
        validate_password(&self.password, 1)
    }
}

#[derive(Debug, Serialize)]
pub struct ApiStar {
    pub id: i64,
    pub name: String,
    pub catalog_id: String,
    pub distance_kpc: String,
    pub velocity_kms: i32,
    pub status: String,
    pub description: String,
    pub image_url: String,
    pub video_url: String,
    pub likes_count: i64,
    pub is_owner: i32,
}

#[derive(Debug, Serialize)]
pub struct ApiStarsList {
    pub items: Vec<ApiStar>,
    pub page: u64,
    pub has_more: bool,
}

fn validate_required_text(
    field: &str,
    value: &str,
    maximum_length: usize,
) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::Validation(format!("поле {field} обязательно")));
    }
    if value.chars().count() > maximum_length {
        return Err(AppError::Validation(format!(
            "поле {field} не должно превышать {maximum_length} символов"
        )));
    }
    if value
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(AppError::Validation(format!(
            "поле {field} содержит недопустимые управляющие символы"
        )));
    }
    Ok(value.to_owned())
}

fn validate_password(password: &str, minimum_length: usize) -> Result<(), AppError> {
    let length = password.chars().count();
    if length < minimum_length || length > PASSWORD_MAX_LENGTH {
        return Err(AppError::Validation(format!(
            "password должен содержать от {minimum_length} до {PASSWORD_MAX_LENGTH} символов"
        )));
    }
    Ok(())
}
