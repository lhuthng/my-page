use dotenvy::dotenv;
use std::env;

use backend::infrastructure::web::server::HTTPServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,tower_http=info"));
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .compact()
        .init();

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let mut server = HTTPServer::new();
    server.set_addr("0.0.0.0");
    server.set_port("3000");
    server.set_db(&db_url);
    server.start().await?;
    Ok(())
}
