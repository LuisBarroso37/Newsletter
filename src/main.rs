use newsletter::configuration::get_configuration;
use newsletter::startup::Application;
use newsletter::telemetry::{get_subscriber, init_subscriber};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Setup logger
    let subscriber = get_subscriber("newsletter".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    // Read configuration file - panic if we can't read it
    let configuration = get_configuration().expect("Failed to read configuration");

    // Build and run server
    let application = Application::build(configuration).await?;
    application.run_until_stopped().await?;

    Ok(())
}
