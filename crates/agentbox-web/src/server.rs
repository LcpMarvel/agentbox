use agentbox_core::config::DEFAULT_WEB_PORT;
use agentbox_db::connection::DbPool;
use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tracing::info;

pub async fn start_server(pool: DbPool) -> anyhow::Result<()> {
    let app = Router::new()
        .nest("/api", crate::api::routes(pool))
        .fallback(crate::assets::serve_static)
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([127, 0, 0, 1], DEFAULT_WEB_PORT));
    info!("Web dashboard listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
