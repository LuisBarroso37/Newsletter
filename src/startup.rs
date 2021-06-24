use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

use crate::routes::{health_check, subscribe};

/// Runs the server
pub fn run(listener: TcpListener, connection_pool: PgPool) -> Result<Server, std::io::Error> {
    // Wrap the database connection pool using web::Data which boils down
    // to an Arc smart pointer
    let connection_pool = web::Data::new(connection_pool);

    // Capture 'connection_pool' from the surrounding environment by using the
    // `move` keyword
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            // `app.data` does not perform an additional layer of wrapping like `data` does
            // `data would add another Arc smart pointer on top of the existing one`
            .app_data(connection_pool.clone()) // Register a pointer copy of the connection pool as part of the application state
    })
    .listen(listener)?
    .run();

    Ok(server)
}
