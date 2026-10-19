use actix_web::{HttpResponse, http::header::LOCATION, web};

use crate::handlers::{
    api::{
        api_create_star, api_delete_star, api_get_draft, api_get_feed, api_get_stars,
        api_like_star, api_publish_star,
    },
    stars::{create_draft, delete_star, get_draft, get_star, get_stars_grid, publish_draft},
    users_api::{api_login, api_logout, api_register_user, auth_page, browser_logout},
};

pub fn config_routes(cfg: &mut web::ServiceConfig) {
    // These aliases must precede /andromeda-stars/{id}.
    for path in [
        "/swagger-ui",
        "/andromeda-stars/swagger-ui",
        "/andromeda-stars/swagger-ui/",
    ] {
        cfg.route(path, web::get().to(swagger_redirect));
    }

    cfg.service(
        web::scope("/api")
            .service(api_get_stars)
            .service(api_get_feed)
            .service(api_get_draft)
            .service(api_create_star)
            .service(api_publish_star)
            .service(api_delete_star)
            .service(api_like_star)
            .service(api_register_user)
            .service(api_login)
            .service(api_logout),
    )
    .service(auth_page)
    .service(browser_logout)
    .service(get_draft)
    .service(create_draft)
    .service(publish_draft)
    .service(delete_star)
    .service(get_stars_grid)
    .service(get_star);
}

async fn swagger_redirect() -> HttpResponse {
    HttpResponse::TemporaryRedirect()
        .insert_header((LOCATION, "/swagger-ui/"))
        .finish()
}
