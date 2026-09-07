use actix_web::{
    web
};

use crate::handlers::stars::{
    get_draft, get_stars, get_stars_grid, get_star
};

pub fn config_routes(cfg: &mut web::ServiceConfig) {
    cfg
        .service(get_draft)
        .service(get_stars)
        .service(get_stars_grid)
        .service(get_star);
}