use std::format;

use actix_web::{
    HttpResponse,
    get,
    http::{
        header::ContentType
    },
    web::{
        Data,
        Path,
        Query
    }
};
use serde::{
    Deserialize,
    Serialize
};
use tera::{
    Context,
    Tera
};

use crate::{
    errors::{
        AppError
    },
    models::{
        AppState,
        Star,
        StarStatus,
        calculate_velocity_kms
    }
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

#[derive(Debug, Serialize)]
struct StarTemplate {
    id: u32,
    name: String,
    catalog_id: String,
    distance_kpc: String,
    velocity_kms: u16,
    status: StarStatus,
    description: String,
    image_url: String,
    video_url: String,
    likes_count: usize,
}

#[get("/stars")]
pub async fn get_stars(
    query: Query<FeedQuery>,
    state: Data<AppState>,
    templates: Data<Tera>,
) -> Result<HttpResponse, AppError> {
    let mut visible_stars: Vec<Star> = Vec::new();

    for star in &state.stars {
        if star.is_visible() {
            visible_stars.push(star.clone());
        }
    }

    if visible_stars.is_empty() {
        return Err(AppError::NotFound(
            "доступные звёзды".to_owned()
        ));
    }

    let mut current_index = 0;

    if query.next.unwrap_or(false) {
        current_index += 1;

        if current_index >= visible_stars.len() {
            current_index = 0;
        }
    }

    let star_template = make_star_template(&visible_stars[current_index]);

    let mut context = Context::new();
    context.insert("star", &star_template);

    let page = templates.render("feed.html", &context)?;

    Ok(
        HttpResponse::Ok()
            .content_type(ContentType::html())
            .body(page)
    )
}

#[get("/stars/{id}")]
pub async fn get_star(
    id: Path<u32>,
    query: Query<FeedQuery>,
    state: Data<AppState>,
    templates: Data<Tera>,
) -> Result<HttpResponse, AppError> {
    let mut visible_stars: Vec<Star> = Vec::new();

    for star in &state.stars {
        if star.is_visible() {
            visible_stars.push(star.clone());
        }
    }

    if visible_stars.is_empty() {
        return Err(AppError::NotFound(
            "доступные звёзды".to_owned()
        ));
    }

    let requested_id = id.into_inner();

    let found_index = visible_stars
        .iter()
        .position(|star| star.id == requested_id);

    let mut current_index = match found_index {
        Some(index) => index,
        None => {
            return Err(AppError::NotFound(
                format!(
                    "звезда с id {} отсутствует или удалена",
                    requested_id
                )
            ));
        }
    };

    if query.next.unwrap_or(false) {
        current_index += 1;

        if current_index >= visible_stars.len() {
            current_index = 0;
        }
    }

    let star_template = make_star_template(&visible_stars[current_index]);

    let mut context = Context::new();
    context.insert("star", &star_template);

    let page = templates.render("feed.html", &context)?;

    Ok(
        HttpResponse::Ok()
            .content_type(ContentType::html())
            .body(page)
    )
}

#[get("/stars/draft")]
pub async fn get_draft(
    state: Data<AppState>,
    templates: Data<Tera>,
) -> Result<HttpResponse, AppError> {
    let draft = state
        .stars
        .iter()
        .find(|star| star.status == StarStatus::Draft);

    let star = match draft {
        Some(star) => star,
        None => {
            return Err(AppError::NotFound("черновик звезды".to_owned()));
        },
    };

    let star_template = make_star_template(star);

    let mut context = Context::new();
    context.insert("star", &star_template);

    let page = templates.render("draft.html", &context)?;

    Ok(
        HttpResponse::Ok()
            .content_type(ContentType::html())
            .body(page)
    )
}

#[get("/stars/grid")]
pub async fn get_stars_grid(
    query: Query<StarsFilter>,
    state: Data<AppState>,
    templates: Data<Tera>,
) -> Result<HttpResponse, AppError> {
    let raw_distance = match &query.distance_kpc {
        Some(value) => value.trim().to_owned(),
        None => String::new(),
    };

    let maximum_distance = if raw_distance.is_empty() {
        None
    } else {
        let normalized_distance = raw_distance.replace(',', ".");
        let parse_result = normalized_distance.parse::<f32>();

        let distance = match parse_result {
            Ok(value) => value,
            Err(_) => {
                return Err(AppError::Validation(
                    "distance_kpc должен быть числом, например 18.4".to_owned()
                ));
            },
        };

        if !distance.is_finite() || distance < 0.0 {
            return Err(AppError::Validation(
                "distance_kpc должен быть конечным неотрицательным числом".to_owned()
            ));
        }

        Some(distance)
    };

    let mut filtered_stars: Vec<StarTemplate> = Vec::new();

    for star in &state.stars {
        if !star.is_visible() {
            continue;
        }

        if let Some(maximum_distance) = maximum_distance {
            if star.distance_kpc > maximum_distance {
                continue;
            }
        }

        filtered_stars.push(make_star_template(star));
    }

    let page_size = 20;
    let page_number = query.page.unwrap_or(1).max(1);
    let page_start = page_number.saturating_sub(1).saturating_mul(page_size);
    let has_more = page_start.saturating_add(page_size) < filtered_stars.len();
    let stars: Vec<StarTemplate> = filtered_stars
        .into_iter()
        .skip(page_start)
        .take(page_size)
        .collect();

    let cover_image_url = match stars.first() {
        Some(first_star) => first_star.image_url.clone(),
        None => String::new(),
    };
    let has_stars = !stars.is_empty();
    let filter_active = maximum_distance.is_some();

    let mut context = Context::new();
    context.insert("has_stars", &has_stars);
    context.insert("stars", &stars);
    context.insert("distance_kpc", &raw_distance);
    context.insert("filter_active", &filter_active);
    context.insert("cover_image_url", &cover_image_url);
    context.insert("next_page", &page_number.saturating_add(1));
    context.insert("has_more", &has_more);

    let page = templates.render("grid.html", &context)?;

    Ok(
        HttpResponse::Ok()
            .content_type(ContentType::html())
            .body(page)
    )
}

fn make_star_template(star: &Star) -> StarTemplate {
    StarTemplate {
        id: star.id,
        name: star.name.clone(),
        catalog_id: star.catalog_id.clone(),
        distance_kpc: format!("{:.1}", star.distance_kpc),
        velocity_kms: calculate_velocity_kms(star.distance_kpc),
        status: star.status.clone(),
        description: star.description.clone(),
        image_url: star.image_url.clone(),
        video_url: star.video_url.clone(),
        likes_count: star.likes_count(),
    }
}
