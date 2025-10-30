use axum::{Router, extract::Path, http::StatusCode, response::IntoResponse, routing::get};
use tokio;

async fn api_handler(Path(text): Path<String>) -> impl IntoResponse {
    let processed_text = text.to_uppercase();

    let response_message = format!("API response for: {}", processed_text);

    (StatusCode::OK, response_message)
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/api/{text}", get(api_handler));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    println!("Server has started!");
    axum::serve(listener, app).await.unwrap();
}
