use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

use crate::configuration::{DatabaseSettings, Settings};
use crate::email_client::EmailClient;
use crate::routes::{confirm, health_check, subscribe};

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
        let server = run(
            listener,
            connection_pool,
            email_client,
            configuration.application.base_url,
        )?;

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

// We need to define a wrapper type in order to retrieve the URL
// in the `subscribe` handler
// Retrieval from the context, in actix-web, is type-based: using
// a raw `String` would expose us to conflicts
#[derive(Debug)]
pub struct ApplicationBaseUrl(pub String);

/// Runs the server
pub fn run(
    listener: TcpListener,
    connection_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
) -> Result<Server, std::io::Error> {
    // Wrap the database connection pool and Email http client using web::Data which boils down
    // to an Arc smart pointer
    let connection_pool = web::Data::new(connection_pool);
    let email_client = web::Data::new(email_client);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));

    // Capture 'connection_pool' from the surrounding environment by using the
    // `move` keyword
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .route("/subscriptions/confirm", web::get().to(confirm))
            // `app.data` does not perform an additional layer of wrapping like `data` does
            // `data would add another Arc smart pointer on top of the existing one`
            .app_data(connection_pool.clone()) // Register a pointer copy of the connection pool as part of the application state
            .app_data(email_client.clone())
            .app_data(base_url.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
