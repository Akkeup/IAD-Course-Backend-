use std::env;

use actix_web::{
    HttpResponse, get,
    http::header::{ContentType, LOCATION},
    post,
    web::{Data, Form, Path, Query},
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, prelude::Decimal,
};
use serde::Deserialize;
use tera::{Context, Tera};
use uuid::Uuid;

use crate::{
    database::{AppDatabase, current_andromeda_user_id},
    errors::AppError,
    models::{
        StarTemplate,
        andromeda_likes::{Column as LikeColumn, Entity as Likes},
        andromeda_stars::{
            ActiveModel as ActiveStar, Column as StarColumn, Entity as Stars, Model as Star,
        },
    },
};

#[derive(Debug, Deserialize)]
pub struct FeedQuery {
    next: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct StarsFilter {
    distance_kpc: Option<String>,
    page: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct DraftForm {
    name: String,
}

#[derive(Debug, Deserialize)]
pub struct PublishForm {
    name: String,
    catalog_id: String,
    distance_kpc: f32,
    velocity_kms: i32,
    description: String,
}

const DEFAULT_IMAGE_URL: &str = "/static/defaults/andromeda-placeholder.svg";
const DEFAULT_VIDEO_URL: &str = "/static/defaults/andromeda-placeholder.mp4";

#[get("/andromeda-stars/{id}")]
pub async fn get_star(
    id: Path<String>,
    query: Query<FeedQuery>,
    database: Data<AppDatabase>,
    templates: Data<Tera>,
) -> Result<HttpResponse, AppError> {
    let requested_value = id.into_inner();
    let requested_id = if requested_value == "first" {
        None
    } else {
        Some(requested_value.parse::<i64>().map_err(|_| {
            AppError::Validation("id звезды должен быть целым числом или first".to_owned())
        })?)
    };

    let star = match requested_id {
        None => first_published_star(&database).await?,
        Some(star_id) if query.next.unwrap_or(false) => {
            match Stars::find()
                .filter(StarColumn::StarStatus.eq("published"))
                .filter(StarColumn::StarId.gt(star_id))
                .order_by_asc(StarColumn::StarId)
                .one(&database.orm)
                .await?
            {
                Some(star) => Some(star),
                None => first_published_star(&database).await?,
            }
        }
        Some(star_id) => {
            Stars::find_by_id(star_id)
                .filter(StarColumn::StarStatus.eq("published"))
                .one(&database.orm)
                .await?
        }
    }
    .ok_or_else(|| {
        AppError::NotFound(format!(
            "звезда {} отсутствует или удалена",
            requested_value
        ))
    })?;

    render_feed(&templates, &database, star).await
}

#[get("/andromeda-stars/draft")]
pub async fn get_draft(
    database: Data<AppDatabase>,
    templates: Data<Tera>,
) -> Result<HttpResponse, AppError> {
    let user_id = current_andromeda_user_id(&database).await?;
    let star = Stars::find()
        .filter(StarColumn::CreatedBy.eq(user_id))
        .filter(StarColumn::StarStatus.eq("draft"))
        .order_by_asc(StarColumn::StarId)
        .one(&database.orm)
        .await?;

    let star = match star {
        Some(star) => {
            let count = likes_count(&database, star.star_id).await?;
            to_template(star, count)
        }
        None => empty_draft_template(),
    };
    let mut context = Context::new();
    context.insert("star", &star);
    let page = templates.render("draft.html", &context)?;

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(page))
}

#[post("/andromeda-stars/draft")]
pub async fn create_draft(
    form: Form<DraftForm>,
    database: Data<AppDatabase>,
) -> Result<HttpResponse, AppError> {
    validate_draft(&form)?;
    let user_id = current_andromeda_user_id(&database).await?;
    let existing_draft = Stars::find()
        .filter(StarColumn::CreatedBy.eq(user_id))
        .filter(StarColumn::StarStatus.eq("draft"))
        .one(&database.orm)
        .await?;
    if existing_draft.is_some() {
        return Ok(HttpResponse::SeeOther()
            .insert_header((LOCATION, "/andromeda-stars/draft"))
            .finish());
    }

    ActiveStar {
        created_by: Set(user_id),
        star_name: Set(form.name.trim().to_owned()),
        star_catalog_id: Set(format!("M31-DRAFT-{}", Uuid::new_v4())),
        distance_kpc: Set(Decimal::ZERO),
        velocity_kms: Set(0),
        star_status: Set("draft".to_owned()),
        star_description: Set(String::new()),
        image_url: Set(String::new()),
        video_url: Set(String::new()),
        ..Default::default()
    }
    .insert(&database.orm)
    .await?;

    Ok(HttpResponse::SeeOther()
        .insert_header((LOCATION, "/andromeda-stars/draft"))
        .finish())
}

#[post("/andromeda-stars/{id}/publish")]
pub async fn publish_draft(
    id: Path<i64>,
    form: Form<PublishForm>,
    database: Data<AppDatabase>,
) -> Result<HttpResponse, AppError> {
    validate_publish(&form)?;
    let user_id = current_andromeda_user_id(&database).await?;
    let star_id = id.into_inner();
    let star = Stars::find_by_id(star_id)
        .filter(StarColumn::CreatedBy.eq(user_id))
        .filter(StarColumn::StarStatus.eq("draft"))
        .one(&database.orm)
        .await?
        .ok_or_else(|| AppError::NotFound("черновик звезды".to_owned()))?;

    let mut star: ActiveStar = star.into();
    star.star_name = Set(form.name.trim().to_owned());
    star.star_catalog_id = Set(form.catalog_id.trim().to_owned());
    star.distance_kpc = Set(decimal_distance(form.distance_kpc)?);
    star.velocity_kms = Set(form.velocity_kms);
    star.star_description = Set(form.description.trim().to_owned());
    star.star_status = Set("published".to_owned());
    star.updated_at = Set(Utc::now().fixed_offset());
    star.update(&database.orm).await?;

    Ok(HttpResponse::SeeOther()
        .insert_header((LOCATION, format!("/andromeda-stars/{}", star_id)))
        .finish())
}

#[post("/andromeda-stars/{id}/delete")]
pub async fn delete_star(
    id: Path<i64>,
    database: Data<AppDatabase>,
) -> Result<HttpResponse, AppError> {
    // По условию лабораторной логическое удаление выполняется чистым SQL, без ORM.
    let result = sqlx::query(
        "UPDATE andromeda_stars SET star_status = 'deleted', updated_at = NOW() WHERE star_id = $1 AND star_status <> 'deleted'",
    )
    .bind(id.into_inner())
    .execute(&database.sql)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("активная карточка звезды".to_owned()));
    }

    Ok(HttpResponse::SeeOther()
        .insert_header((LOCATION, "/andromeda-stars/grid"))
        .finish())
}

#[get("/andromeda-stars/grid")]
pub async fn get_stars_grid(
    query: Query<StarsFilter>,
    database: Data<AppDatabase>,
    templates: Data<Tera>,
) -> Result<HttpResponse, AppError> {
    let raw_distance = query
        .distance_kpc
        .clone()
        .unwrap_or_default()
        .trim()
        .to_owned();
    let maximum_distance = if raw_distance.is_empty() {
        None
    } else {
        let distance = raw_distance.replace(',', ".").parse::<f32>().map_err(|_| {
            AppError::Validation("distance_kpc должен быть числом, например 18.4".to_owned())
        })?;
        Some(decimal_distance(distance)?)
    };

    let page_number = query.page.unwrap_or(1).max(1);
    let page_size = 20_u64;
    let page_start = (page_number.saturating_sub(1) as u64) * page_size;
    let mut stars_query = Stars::find().filter(StarColumn::StarStatus.eq("published"));
    if let Some(distance) = maximum_distance {
        stars_query = stars_query.filter(StarColumn::DistanceKpc.lte(distance));
    }
    let stars = stars_query
        .order_by_asc(StarColumn::StarId)
        .offset(page_start)
        .limit(page_size)
        .all(&database.orm)
        .await?;

    let has_more = stars.len() == page_size as usize;
    let mut star_templates = Vec::with_capacity(stars.len());
    for star in stars {
        let count = likes_count(&database, star.star_id).await?;
        star_templates.push(to_template(star, count));
    }
    let cover_image_url = star_templates
        .first()
        .map(|star| star.image_url.clone())
        .unwrap_or_default();

    let mut context = Context::new();
    context.insert("has_stars", &!star_templates.is_empty());
    context.insert("stars", &star_templates);
    context.insert("distance_kpc", &raw_distance);
    context.insert("filter_active", &maximum_distance.is_some());
    context.insert("cover_image_url", &cover_image_url);
    context.insert("next_page", &page_number.saturating_add(1));
    context.insert("has_more", &has_more);

    let page = templates.render("grid.html", &context)?;
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(page))
}

async fn first_published_star(database: &AppDatabase) -> Result<Option<Star>, AppError> {
    Ok(Stars::find()
        .filter(StarColumn::StarStatus.eq("published"))
        .order_by_asc(StarColumn::StarId)
        .one(&database.orm)
        .await?)
}

async fn likes_count(database: &AppDatabase, star_id: i64) -> Result<i64, AppError> {
    let count = Likes::find()
        .filter(LikeColumn::StarId.eq(star_id))
        .count(&database.orm)
        .await?;
    Ok(count as i64)
}

async fn render_feed(
    templates: &Tera,
    database: &AppDatabase,
    star: Star,
) -> Result<HttpResponse, AppError> {
    let count = likes_count(database, star.star_id).await?;
    let mut context = Context::new();
    context.insert("star", &to_template(star, count));
    let page = templates.render("feed.html", &context)?;
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(page))
}

fn to_template(star: Star, likes_count: i64) -> StarTemplate {
    StarTemplate {
        id: star.star_id,
        name: star.star_name,
        catalog_id: star.star_catalog_id,
        distance_kpc: star.distance_kpc.round_dp(1).to_string(),
        velocity_kms: star.velocity_kms,
        status: star.star_status,
        description: star.star_description,
        image_url: media_url(&star.image_url, DEFAULT_IMAGE_URL),
        video_url: media_url(&star.video_url, DEFAULT_VIDEO_URL),
        likes_count,
    }
}

fn empty_draft_template() -> StarTemplate {
    StarTemplate {
        id: 0,
        name: String::new(),
        catalog_id: String::new(),
        distance_kpc: "0.0".to_owned(),
        velocity_kms: 0,
        status: "new".to_owned(),
        description: String::new(),
        image_url: DEFAULT_IMAGE_URL.to_owned(),
        video_url: DEFAULT_VIDEO_URL.to_owned(),
        likes_count: 0,
    }
}

fn media_url(value: &str, fallback: &str) -> String {
    if value.is_empty() || value.starts_with("http://") || value.starts_with("https://") {
        return if value.is_empty() {
            fallback.to_owned()
        } else {
            value.to_owned()
        };
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

fn decimal_distance(value: f32) -> Result<Decimal, AppError> {
    if !value.is_finite() || value < 0.0 {
        return Err(AppError::Validation(
            "distance_kpc должен быть конечным неотрицательным числом".to_owned(),
        ));
    }
    Decimal::from_f32_retain(value)
        .ok_or_else(|| AppError::Validation("не удалось преобразовать distance_kpc".to_owned()))
}

fn validate_draft(form: &DraftForm) -> Result<(), AppError> {
    if form.name.trim().is_empty() {
        return Err(AppError::Validation(
            "название звезды обязательно".to_owned(),
        ));
    }
    Ok(())
}

fn validate_publish(form: &PublishForm) -> Result<(), AppError> {
    if form.name.trim().is_empty() || form.catalog_id.trim().is_empty() {
        return Err(AppError::Validation(
            "название и каталожный ID обязательны".to_owned(),
        ));
    }
    decimal_distance(form.distance_kpc)?;
    if form.velocity_kms < 0 {
        return Err(AppError::Validation(
            "velocity_kms должен быть неотрицательным числом".to_owned(),
        ));
    }
    if form.description.trim().is_empty() {
        return Err(AppError::Validation(
            "краткое описание обязательно".to_owned(),
        ));
    }
    Ok(())
}
