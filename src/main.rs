mod errors;
mod handlers;
mod models;
mod routes;

use std::{
    env,
    format,
    io
};

use actix_files::{
    Files
};
use actix_web::{
    App,
    HttpServer,
    HttpResponse,
    web::{
        Data
    },
    get
};
use tera::{
    Tera
};

use crate::{
    models::{
        AppState
    },
    routes::{
        config_routes
    },
    errors::{
        AppError
    }
};

#[actix_web::main]
async fn main() -> io::Result<()> {
    dotenvy::dotenv().ok();

    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_owned());
    let bind_address = format!("{}:{}", host, port);

    let templates_glob = format!("{}/templates/**/*", env!("CARGO_MANIFEST_DIR"));
    let mut templates = Tera::new();
    templates
        .load_from_glob(&templates_glob)
        .map_err(|error| io::Error::other(format!("Ошибка загрузки Tera: {}", error)))?;

    let templates = Data::new(templates);
    let state = Data::new(AppState::from_env());
    let static_directory = format!("{}/static", env!("CARGO_MANIFEST_DIR"));

    println!("M31 Stars запущен: http://{}/stars", bind_address);

    HttpServer::new(move || {
        App::new()
            .app_data(templates.clone())
            .app_data(state.clone())
            .configure(config_routes)
            .service(
                Files::new("/static", static_directory.clone())
                    .prefer_utf8(true)
            )
    })
    .bind(bind_address.as_str())?
    .run()
    .await
}

#[get("/")]
async fn mainpage() -> Result<HttpResponse, AppError> {
    Ok(HttpResponse::Ok().body(format!("Scary Spooky Skeleton")))
}   