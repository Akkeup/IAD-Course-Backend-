use actix_multipart::form::MultipartForm;
use actix_web::{
    HttpResponse, delete, get, post, put,
    web::{Data, Json, Path, Query},
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, prelude::Decimal, sea_query::OnConflict,
};
use uuid::Uuid;

use crate::{
    database::{AppDatabase, current_user_id},
    errors::AppError,
    handlers::{
        api_dto::{
            ApiCreateStarForm, ApiFeedQuery, ApiLikeRequest, ApiPublishRequest, ApiStar,
            ApiStarsFilter, ApiStarsList, page_offset, validate_resource_id,
        },
        storage::{public_media_url, remove_media, upload_media},
    },
    models::{
        andromeda_likes::{self, Column as LikeColumn, Entity as Likes},
        andromeda_stars::{
            ActiveModel as ActiveStar, Column as StarColumn, Entity as Stars, Model as Star,
        },
    },
};

const PAGE_SIZE: u64 = 20;

#[get("/andromeda-stars")]
pub async fn api_get_stars(
    query: Query<ApiStarsFilter>,
    database: Data<AppDatabase>,
) -> Result<HttpResponse, AppError> {
    let user_id = current_user_id(&database).await?;
    let maximum_distance = parse_distance(query.distance_kpc.as_deref())?;
    let (page, offset) = page_offset(query.page, PAGE_SIZE)?;

    let mut stars_query = Stars::find().filter(StarColumn::StarStatus.eq("published"));
    if let Some(distance) = maximum_distance {
        stars_query = stars_query.filter(StarColumn::DistanceKpc.lte(distance));
    }

    let stars = stars_query
        .order_by_asc(StarColumn::StarId)
        .offset(offset)
        .limit(PAGE_SIZE)
        .all(&database.orm)
        .await?;
    let has_more = stars.len() == PAGE_SIZE as usize;
    let mut items = Vec::with_capacity(stars.len());
    for star in stars {
        items.push(api_star(&database, user_id, star).await?);
    }

    Ok(HttpResponse::Ok().json(ApiStarsList {
        items,
        page,
        has_more,
    }))
}

#[get("/andromeda-stars/feed")]
pub async fn api_get_feed(
    query: Query<ApiFeedQuery>,
    database: Data<AppDatabase>,
) -> Result<HttpResponse, AppError> {
    let user_id = current_user_id(&database).await?;
    let requested_id = query.id.map(validate_resource_id).transpose()?;
    let star = match (requested_id, query.next.unwrap_or(false)) {
        (None, _) => first_published_star(&database).await?,
        (Some(star_id), true) => next_published_star(&database, star_id).await?,
        (Some(star_id), false) => {
            Stars::find_by_id(star_id)
                .filter(StarColumn::StarStatus.eq("published"))
                .one(&database.orm)
                .await?
        }
    }
    .ok_or_else(|| AppError::NotFound("опубликованная звезда".to_owned()))?;

    Ok(HttpResponse::Ok().json(api_star(&database, user_id, star).await?))
}

#[get("/andromeda-stars/draft")]
pub async fn api_get_draft(database: Data<AppDatabase>) -> Result<HttpResponse, AppError> {
    let user_id = current_user_id(&database).await?;
    let star = Stars::find()
        .filter(StarColumn::CreatedBy.eq(user_id))
        .filter(StarColumn::StarStatus.eq("draft"))
        .one(&database.orm)
        .await?
        .ok_or_else(|| AppError::NotFound("черновик текущего пользователя".to_owned()))?;

    Ok(HttpResponse::Ok().json(api_star(&database, user_id, star).await?))
}

#[post("/andromeda-stars")]
pub async fn api_create_star(
    MultipartForm(form): MultipartForm<ApiCreateStarForm>,
    database: Data<AppDatabase>,
) -> Result<HttpResponse, AppError> {
    let user_id = current_user_id(&database).await?;
    let name = form.validated_name()?;

    let existing_draft = Stars::find()
        .filter(StarColumn::CreatedBy.eq(user_id))
        .filter(StarColumn::StarStatus.eq("draft"))
        .one(&database.orm)
        .await?;
    if existing_draft.is_some() {
        return Err(AppError::Validation(
            "у текущего пользователя уже есть черновик".to_owned(),
        ));
    }

    let media = upload_media(&form).await?;
    let inserted = ActiveStar {
        created_by: Set(user_id),
        star_name: Set(name),
        star_catalog_id: Set(format!("M31-DRAFT-{}", Uuid::new_v4())),
        distance_kpc: Set(Decimal::ZERO),
        velocity_kms: Set(0),
        star_status: Set("draft".to_owned()),
        star_description: Set(String::new()),
        image_url: Set(media.image_name.clone()),
        video_url: Set(media.video_name.clone()),
        ..Default::default()
    }
    .insert(&database.orm)
    .await;

    match inserted {
        Ok(_) => {}
        Err(error) => {
            remove_media(&media).await;
            return Err(error.into());
        }
    }

    Ok(HttpResponse::Created().finish())
}

#[put("/andromeda-stars/{id}/publish")]
pub async fn api_publish_star(
    id: Path<i64>,
    payload: Json<ApiPublishRequest>,
    database: Data<AppDatabase>,
) -> Result<HttpResponse, AppError> {
    let payload = payload.validate()?;
    let user_id = current_user_id(&database).await?;
    let star_id = validate_resource_id(id.into_inner())?;
    let star = Stars::find_by_id(star_id)
        .filter(StarColumn::CreatedBy.eq(user_id))
        .filter(StarColumn::StarStatus.eq("draft"))
        .one(&database.orm)
        .await?
        .ok_or_else(|| AppError::NotFound("черновик текущего пользователя".to_owned()))?;

    let mut star: ActiveStar = star.into();
    star.star_name = Set(payload.name);
    star.star_catalog_id = Set(payload.catalog_id);
    star.distance_kpc = Set(decimal_distance(payload.distance_kpc)?);
    star.velocity_kms = Set(payload.velocity_kms);
    star.star_description = Set(payload.description);
    let formed_at = Utc::now().fixed_offset();
    star.star_status = Set("published".to_owned());
    star.updated_at = Set(formed_at);
    star.formed_at = Set(Some(formed_at));
    star.update(&database.orm).await?;

    Ok(HttpResponse::NoContent().finish())
}

#[delete("/andromeda-stars/{id}")]
pub async fn api_delete_star(
    id: Path<i64>,
    database: Data<AppDatabase>,
) -> Result<HttpResponse, AppError> {
    let user_id = current_user_id(&database).await?;
    let star_id = validate_resource_id(id.into_inner())?;
    let star = Stars::find_by_id(star_id)
        .filter(StarColumn::CreatedBy.eq(user_id))
        .filter(StarColumn::StarStatus.is_in(["draft", "published"]))
        .one(&database.orm)
        .await?
        .ok_or_else(|| AppError::NotFound("активная звезда текущего пользователя".to_owned()))?;

    let mut star: ActiveStar = star.into();
    star.star_status = Set("deleted".to_owned());
    star.updated_at = Set(Utc::now().fixed_offset());
    star.update(&database.orm).await?;

    Ok(HttpResponse::NoContent().finish())
}

#[post("/andromeda-stars/{id}/like")]
pub async fn api_like_star(
    id: Path<i64>,
    payload: Json<ApiLikeRequest>,
    database: Data<AppDatabase>,
) -> Result<HttpResponse, AppError> {
    let like_value = payload.validate()?;

    let user_id = current_user_id(&database).await?;
    let star_id = validate_resource_id(id.into_inner())?;
    Stars::find_by_id(star_id)
        .filter(StarColumn::StarStatus.eq("published"))
        .one(&database.orm)
        .await?
        .ok_or_else(|| AppError::NotFound("опубликованная звезда".to_owned()))?;

    if like_value == 1 {
        Likes::insert(andromeda_likes::ActiveModel {
            user_id: Set(user_id),
            star_id: Set(star_id),
            ..Default::default()
        })
        .on_conflict(
            OnConflict::columns([LikeColumn::UserId, LikeColumn::StarId])
                .do_nothing()
                .to_owned(),
        )
        .do_nothing()
        .exec(&database.orm)
        .await?;
    } else {
        Likes::delete_many()
            .filter(LikeColumn::UserId.eq(user_id))
            .filter(LikeColumn::StarId.eq(star_id))
            .exec(&database.orm)
            .await?;
    }

    Ok(HttpResponse::NoContent().finish())
}

async fn first_published_star(database: &AppDatabase) -> Result<Option<Star>, AppError> {
    Ok(Stars::find()
        .filter(StarColumn::StarStatus.eq("published"))
        .order_by_asc(StarColumn::StarId)
        .one(&database.orm)
        .await?)
}

async fn next_published_star(
    database: &AppDatabase,
    star_id: i64,
) -> Result<Option<Star>, AppError> {
    let next = Stars::find()
        .filter(StarColumn::StarStatus.eq("published"))
        .filter(StarColumn::StarId.gt(star_id))
        .order_by_asc(StarColumn::StarId)
        .one(&database.orm)
        .await?;

    match next {
        Some(star) => Ok(Some(star)),
        None => first_published_star(database).await,
    }
}

async fn api_star(database: &AppDatabase, user_id: i64, star: Star) -> Result<ApiStar, AppError> {
    let likes_count = Likes::find()
        .filter(LikeColumn::StarId.eq(star.star_id))
        .count(&database.orm)
        .await?;

    Ok(ApiStar {
        id: star.star_id,
        name: star.star_name,
        catalog_id: star.star_catalog_id,
        distance_kpc: star.distance_kpc.round_dp(1).to_string(),
        velocity_kms: star.velocity_kms,
        status: star.star_status,
        description: star.star_description,
        image_url: public_media_url(&star.image_url),
        video_url: public_media_url(&star.video_url),
        likes_count: likes_count as i64,
        is_owner: i32::from(star.created_by == user_id),
    })
}

fn parse_distance(raw_distance: Option<&str>) -> Result<Option<Decimal>, AppError> {
    let Some(raw_distance) = raw_distance else {
        return Ok(None);
    };
    if raw_distance.trim().is_empty() {
        return Ok(None);
    }
    let distance = raw_distance
        .trim()
        .replace(',', ".")
        .parse::<f32>()
        .map_err(|_| AppError::Validation("distance_kpc должен быть числом".to_owned()))?;
    decimal_distance(distance).map(Some)
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
