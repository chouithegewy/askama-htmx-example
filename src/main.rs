//! A small reading list, as an example of a server-rendered Rust site: actix-web serves the
//! pages, askama compiles the HTML templates in "templates/" into Rust, and htmx makes them
//! feel like an app without a JavaScript framework.
//!
//! The books are kept in memory so the example runs with nothing to set up; a real site would
//! keep them in a database (sqlx with Postgres or SQLite works well alongside this).

mod books;
mod error;
mod nav;
mod routes;

use std::sync::Mutex;

use actix_files::Files;
use actix_web::{App, HttpServer, middleware, web};

use crate::books::Library;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    let address = std::env::var("ADDRESS").unwrap_or_else(|_| "127.0.0.1:8080".to_owned());
    // One library shared by every worker thread. `web::Data` is an `Arc` already, so the
    // only thing to add is the lock.
    let library = web::Data::new(Mutex::new(Library::with_examples()));

    log::info!("listening on http://{address}");
    HttpServer::new(move || {
        App::new()
            .app_data(library.clone())
            .wrap(middleware::Logger::default())
            .configure(routes::configure)
            .service(Files::new("/static", "static"))
            .default_service(web::to(routes::not_found))
    })
    .bind(address)?
    .run()
    .await
}
