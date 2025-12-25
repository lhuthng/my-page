use dotenvy::dotenv;
use tokio;

use crate::infrastructure::web::server::HTTPServer;

mod application;
mod domain;
mod helper;
mod infrastructure;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let mut server = HTTPServer::new();
    server.set_addr("0.0.0.0");
    server.set_port("3000");
    server.set_db("sqlite:data/blog.db");
    server.start().await?;
    Ok(())
}
