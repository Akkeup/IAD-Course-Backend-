use std::env;

use actix_multipart::form::tempfile::TempFile;
use s3::{Bucket, Region, creds::Credentials};
use uuid::Uuid;

use crate::{errors::AppError, handlers::api_dto::ApiCreateStarForm};

pub struct UploadedMedia {
    pub image_name: String,
    pub video_name: String,
}

pub async fn upload_media(form: &ApiCreateStarForm) -> Result<UploadedMedia, AppError> {
    let image_bytes = read_file(&form.image).await?;
    let video_bytes = read_file(&form.video).await?;
    let (image_extension, image_content_type) = media_format(&form.image, &image_bytes, "image")?;
    let (video_extension, video_content_type) = media_format(&form.video, &video_bytes, "video")?;
    let bucket = minio_bucket()?;
    let image_name = format!("images/{}.{}", Uuid::new_v4(), image_extension);
    let video_name = format!("videos/{}.{}", Uuid::new_v4(), video_extension);

    bucket
        .put_object_with_content_type(&image_name, &image_bytes, image_content_type)
        .await
        .map_err(|error| AppError::Database(error.to_string()))?;

    if let Err(error) = bucket
        .put_object_with_content_type(&video_name, &video_bytes, video_content_type)
        .await
    {
        let _ = bucket.delete_object(&image_name).await;
        return Err(AppError::Database(error.to_string()));
    }

    Ok(UploadedMedia {
        image_name,
        video_name,
    })
}

pub async fn remove_media(media: &UploadedMedia) {
    let Ok(bucket) = minio_bucket() else {
        return;
    };
    let _ = bucket.delete_object(&media.image_name).await;
    let _ = bucket.delete_object(&media.video_name).await;
}

pub fn public_media_url(value: &str) -> String {
    if value.is_empty() || value.starts_with("http://") || value.starts_with("https://") {
        return value.to_owned();
    }

    let base = env::var("ANDROMEDA_MINIO_PUBLIC_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:9000".to_owned());
    let bucket = env::var("ANDROMEDA_MINIO_BUCKET").unwrap_or_else(|_| "andromeda".to_owned());
    format!(
        "{}/{}/{}",
        base.trim_end_matches('/'),
        bucket.trim_matches('/'),
        value.trim_start_matches('/')
    )
}

async fn read_file(file: &TempFile) -> Result<Vec<u8>, AppError> {
    tokio::fs::read(file.file.path())
        .await
        .map_err(|error| AppError::Database(error.to_string()))
}

fn media_format(
    file: &TempFile,
    bytes: &[u8],
    expected: &str,
) -> Result<(&'static str, &'static str), AppError> {
    let content_type = file
        .content_type
        .as_ref()
        .map(|content_type| content_type.as_ref())
        .ok_or_else(|| AppError::Validation(format!("не указан тип файла {expected}")))?;

    validate_media_format(content_type, bytes, expected)
}

fn validate_media_format(
    content_type: &str,
    bytes: &[u8],
    expected: &str,
) -> Result<(&'static str, &'static str), AppError> {
    match (expected, content_type) {
        ("image", "image/jpeg") if bytes.starts_with(&[0xff, 0xd8, 0xff]) => {
            Ok(("jpg", "image/jpeg"))
        }
        ("image", "image/png")
            if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) =>
        {
            Ok(("png", "image/png"))
        }
        ("image", "image/webp")
            if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") =>
        {
            Ok(("webp", "image/webp"))
        }
        ("video", "video/mp4") if bytes.get(4..8) == Some(b"ftyp") => Ok(("mp4", "video/mp4")),
        ("image", "image/jpeg" | "image/png" | "image/webp") | ("video", "video/mp4") => {
            Err(AppError::Validation(format!(
                "содержимое файла {expected} не соответствует заявленному типу"
            )))
        }
        _ => Err(AppError::Validation(format!(
            "недопустимый тип файла {expected}: {content_type}"
        ))),
    }
}

fn minio_bucket() -> Result<Box<Bucket>, AppError> {
    let endpoint = env::var("ANDROMEDA_MINIO_ENDPOINT")
        .or_else(|_| env::var("ANDROMEDA_MINIO_PUBLIC_URL"))
        .unwrap_or_else(|_| "http://127.0.0.1:9000".to_owned());
    let bucket_name = env::var("ANDROMEDA_MINIO_BUCKET").unwrap_or_else(|_| "andromeda".to_owned());
    let access_key = env::var("ANDROMEDA_MINIO_ROOT_USER").unwrap_or_else(|_| "root".to_owned());
    let secret_key =
        env::var("ANDROMEDA_MINIO_ROOT_PASSWORD").unwrap_or_else(|_| "rootpassword".to_owned());
    let credentials = Credentials::new(Some(&access_key), Some(&secret_key), None, None, None)
        .map_err(|error| AppError::Database(error.to_string()))?;

    Bucket::new(
        &bucket_name,
        Region::Custom {
            region: "us-east-1".to_owned(),
            endpoint,
        },
        credentials,
    )
    .map(|bucket| bucket.with_path_style())
    .map_err(|error| AppError::Database(error.to_string()))
}
