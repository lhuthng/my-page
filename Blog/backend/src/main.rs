use dotenvy::dotenv;
use tokio;

use crate::infrastructure::web::server::HTTPServer;

mod application;
mod domain;
mod infrastructure;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    println!("Server starting on http://127.0.0.1:3000");
    let server = HTTPServer::new("127.0.0.1:3000", "sqlite:data/blog.db");
    server.start().await?;
    Ok(())
}
