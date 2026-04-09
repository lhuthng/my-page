use dotenvy::dotenv;
use std::env;

use crate::infrastructure::web::server::HTTPServer;

mod application;
mod domain;
mod helper;
mod infrastructure;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data/blog.db".to_string());

    let mut server = HTTPServer::new();
    server.set_addr("0.0.0.0");
    server.set_port("3000");
    server.set_db(&db_url);
    server.start().await?;
    Ok(())
}
