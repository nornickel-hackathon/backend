//! Запуск axum-платформы на порту 8080 (CONTRACTS.md).

use platform::state::AppState;

#[tokio::main]
async fn main() {
    let state = AppState::from_env();
    let app = platform::build_router(state);

    let addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind platform address");
    println!(
        "platform listening on http://{}",
        listener.local_addr().expect("local addr")
    );
    axum::serve(listener, app).await.expect("serve");
}
