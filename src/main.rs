mod database;
mod errors;
mod handlers;
mod models;
mod routes;

use std::{env, format, io};

use actix_files::Files;
use actix_web::{App, HttpServer, web::Data};
use tera::Tera;

use crate::{database::connect, routes::config_routes};

#[actix_web::main]
async fn main() -> io::Result<()> {
    dotenvy::dotenv().ok();

    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_owned());
    let bind_address = format!("{}:{}", host, port);
    let database_pool = connect().await?;

    let templates_glob = format!("{}/templates/**/*", env!("CARGO_MANIFEST_DIR"));
    let mut templates = Tera::new();
    templates
        .load_from_glob(&templates_glob)
        .map_err(|error| io::Error::other(format!("Ошибка загрузки Tera: {}", error)))?;

    let templates = Data::new(templates);
    let database_pool = Data::new(database_pool);
    let static_directory = format!("{}/static", env!("CARGO_MANIFEST_DIR"));

    println!(
        "Andromeda Stars запущен: http://{}/andromeda-stars/first",
        bind_address
    );

    HttpServer::new(move || {
        App::new()
            .app_data(templates.clone())
            .app_data(database_pool.clone())
            .configure(config_routes)
            .service(Files::new("/static", static_directory.clone()).prefer_utf8(true))
    })
    .bind(bind_address.as_str())?
    .run()
    .await
}
