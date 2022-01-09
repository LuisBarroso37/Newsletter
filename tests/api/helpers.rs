use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHasher, Version};
use newsletter::configuration::{get_configuration, DatabaseSettings};
use newsletter::startup::{get_connection_pool, Application};
use newsletter::telemetry::{get_subscriber, init_subscriber};
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;

// Ensure that the `tracing` stack is only initialised once using `once_cell`
static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    }
});

/// Confirmation links embedded in the request to the email API
pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

pub struct TestApp {
    pub address: String,
    pub connection_pool: PgPool,
    pub email_server: MockServer,
    pub port: u16,
    pub test_user: TestUser,
    pub db_name: String,
    pub api_client: reqwest::Client,
}

impl TestApp {
    /// Make POST request to /subscriptions
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request")
    }

    /// Make POST request to /newsletters
    pub async fn post_newsletters(&self, body: serde_json::Value) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/newsletters", &self.address))
            .basic_auth(&self.test_user.username, Some(&self.test_user.password)) // Random credentials
            .json(&body)
            .send()
            .await
            .expect("Failed to execute request")
    }

    /// Make POST request to /login
    pub async fn post_login<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{}/login", &self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to execute request")
    }

    /// Make GET request to /login
    pub async fn get_login(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/login", &self.address))
            .send()
            .await
            .expect("Failed to execute request")
    }

    /// Make GET request to /admin/dashboard
    pub async fn get_admin_dashboard(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/admin/dashboard", &self.address))
            .send()
            .await
            .expect("Failed to execute request")
    }

    /// Make GET request to /admin/password
    pub async fn get_change_password(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/admin/password", &self.address))
            .send()
            .await
            .expect("Failed to execute request")
    }

    /// Make POST request to /admin/password
    pub async fn post_change_password<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{}/admin/password", &self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to execute request")
    }

    /// Make POST request to /admin/logout
    pub async fn post_logout(&self) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/admin/logout", &self.address))
            .send()
            .await
            .expect("Failed to execute request")
    }

    /// Extract the confirmation links embedded in the request to the email API
    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        // Parse the body as JSON, starting from raw bytes
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        // Extract the link from the request body
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|link| *link.kind() == linkify::LinkKind::Url)
                .collect();

            assert_eq!(links.len(), 1);

            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();

            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");

            // Rewrite the URL to include the port
            confirmation_link.set_port(Some(self.port)).unwrap();

            confirmation_link
        };

        let html = get_link(body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(body["TextBody"].as_str().unwrap());

        ConfirmationLinks { html, plain_text }
    }

    /// Teardown database created for test
    pub async fn cleanup(&self) {
        let mut conn =
            PgConnection::connect("postgres://postgres:password@localhost:5432/newsletter")
                .await
                .expect("Failed to connect to Postgres");

        // In Postgres, it is necessary to disconnect users before deleting the database
        conn.execute(
            format!(
                r#"SELECT pg_terminate_backend(pid) 
                FROM pg_stat_activity WHERE datname = '{}'
                AND pid <> pg_backend_pid();"#,
                &self.db_name
            )
            .as_str(),
        )
        .await
        .expect("Failed to disconnect users from Postgres");

        // Delete the database
        conn.execute(format!(r#"DROP DATABASE "{}";"#, &self.db_name).as_str())
            .await
            .unwrap_or_else(|_| panic!("Failed to delete database {}", &self.db_name));
    }
}

// Launch our application in the background
// Trying to bind to port 0 will trigger an OS scan for an available
// port which will then be bound to the application
pub async fn spawn_app() -> TestApp {
    // Setup logger
    // The code in `TRACING` is executed only the first time.
    // All other calls will skip execution.
    Lazy::force(&TRACING);

    // Launch a mock server to stand in for Postmark's API
    let email_server = MockServer::start().await;

    // Randomise configuration to ensure test isolation
    let configuration = {
        let mut c = get_configuration().expect("Failed to read configuration");

        // Use a different database for each test case
        c.database.database_name = Uuid::new_v4().to_string();

        // Use a random OS port
        c.application.port = 0;

        // Use the mock server as email API
        c.email_client.base_url = email_server.uri();

        c
    };

    // Create and migrate the database
    configure_database(&configuration.database).await;

    // Get test database name
    let db_name = &configuration.database.database_name;

    // Build server
    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build application");

    // Get the port before spawning the application
    let application_port = application.port();

    // Create HTTP client to make requests to our API
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .cookie_store(true)
        .build()
        .unwrap();

    // Launch the server as a background task
    let _ = tokio::spawn(application.run_until_stopped());

    // Create test application
    let test_app = TestApp {
        address: format!("http://localhost:{}", application_port),
        port: application_port,
        connection_pool: get_connection_pool(&configuration.database),
        email_server,
        test_user: TestUser::generate(),
        db_name: db_name.to_string(),
        api_client: client,
    };

    // Create a user for authentication in POST /newsletters
    test_app.test_user.store(&test_app.connection_pool).await;

    // Return the TestApp struct to the caller
    test_app
}

async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // Create database
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to Postgres");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database");

    // Migrate database
    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to Postgres");

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    connection_pool
}

pub struct TestUser {
    pub id: Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    pub fn generate() -> Self {
        Self {
            id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }

    async fn store(&self, connection_pool: &PgPool) {
        // Generate salt
        let salt = SaltString::generate(&mut rand::thread_rng());

        // Hash password
        let password_hash = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(15000, 2, 1, None).unwrap(),
        )
        .hash_password(self.password.as_bytes(), &salt)
        .unwrap()
        .to_string();

        // Save user in database
        sqlx::query!(
            "INSERT INTO users (id, username, password) VALUES ($1, $2, $3)",
            self.id,
            self.username,
            password_hash
        )
        .execute(connection_pool)
        .await
        .expect("Failed to create test user");
    }
}
