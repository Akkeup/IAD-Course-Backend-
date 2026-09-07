use std::fmt::{
    self,
    Display,
    Formatter
};

use actix_web::{
    http::{
        StatusCode
    },
    ResponseError
};

#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    Validation(String),
    Template(tera::Error),
}

impl Display for AppError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            AppError::NotFound(message) => write!(formatter, "Не найдено: {}", message),
            AppError::Validation(message) => write!(formatter, "Ошибка параметров: {}", message),
            AppError::Template(error) => write!(formatter, "Ошибка шаблона: {}", error),
        }
    }
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::Template(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<tera::Error> for AppError {
    fn from(error: tera::Error) -> Self {
        AppError::Template(error)
    }
}
