use axum::{Router, routing::get, routing::post};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/api", get(api_get))
        .route("/api", post(api_post))
        .fallback_service(ServeDir::new("static"));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn api_get() -> &'static str {
    "Hello from GET!"
}

async fn api_post() -> &'static str {
    "Hello from POST!"
}
