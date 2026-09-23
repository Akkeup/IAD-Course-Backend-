use actix_web::web;

use crate::handlers::stars::{
    create_draft, delete_star, get_draft, get_star, get_stars_grid, publish_draft,
};

pub fn config_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(get_draft)
        .service(create_draft)
        .service(publish_draft)
        .service(delete_star)
        .service(get_stars_grid)
        .service(get_star);
}
