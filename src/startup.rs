use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

use crate::configuration::{DatabaseSettings, Settings};
use crate::email_client::EmailClient;
use crate::routes::{health_check, subscribe};

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    /// Build the server
    pub async fn build(configuration: Settings) -> Result<Self, std::io::Error> {
        // Create Postgres database connection pool
        let connection_pool = get_connection_pool(&configuration.database)
            .await
            .expect("Failed to connect to Postgres");

        // Build an email HTTP client
        let sender_email = configuration
            .email_client
            .sender()
            .expect("Invalid sender email address");
        let email_client = EmailClient::new(
            configuration.email_client.base_url,
            sender_email,
            configuration.email_client.authorization_token,
        );

        // Create listener for port acquired from config file
        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();

        // Run server
        let server = run(listener, connection_pool, email_client)?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    // This function only returns when the application is stopped
    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub async fn get_connection_pool(configuration: &DatabaseSettings) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .connect_timeout(std::time::Duration::from_secs(2))
        .connect_with(configuration.with_db())
        .await
}

/// Runs the server
pub fn run(
    listener: TcpListener,
    connection_pool: PgPool,
    email_client: EmailClient,
) -> Result<Server, std::io::Error> {
    // Wrap the database connection pool and Email http client using web::Data which boils down
    // to an Arc smart pointer
    let connection_pool = web::Data::new(connection_pool);
    let email_client = web::Data::new(email_client);

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
            .app_data(email_client.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
