//! Запуск axum-платформы на порту 8080 (CONTRACTS.md).

use platform::state::AppState;

#[tokio::main]
async fn main() {
    let state = AppState::from_env();
    let app = platform::build_router(state);

    let addr = "127.0.0.1:8080";
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind 127.0.0.1:8080");
    println!("platform listening on http://{addr}");
    axum::serve(listener, app).await.expect("serve");
}
