use anyhow::Result;
use axum::middleware;
use axum::routing::IntoMakeService;
use axum::routing::{get, post};
use axum::{Router, Server};
use hyper::server::conn::AddrIncoming;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::app_state::AppState;
use crate::auth::{api_auth, django_session_auth};
use crate::debug::print_request_response;
use crate::handlers;
use crate::shutdown;

pub struct AppServer {
    server: Server<AddrIncoming, IntoMakeService<Router>>,
    shutdown_rx: shutdown::Receiver,
}

impl AppServer {
    pub fn new(address: &SocketAddr, state: Arc<AppState>) -> Result<AppServer> {
        let shutdown_rx = state.shutdown_rx.clone();

        let app = Router::new()
            .route("/", get(handlers::get_root))
            .route("/notify_tornado", post(handlers::post_notify_tornado))
            .route(
                "/json/events",
                get(handlers::get_events)
                    .delete(handlers::delete_events)
                    .route_layer(middleware::map_request_with_state(
                        state.clone(),
                        django_session_auth,
                    )),
            )
            .route(
                "/api/v1/events",
                get(handlers::get_events)
                    .delete(handlers::delete_events)
                    .route_layer(middleware::map_request_with_state(state.clone(), api_auth)),
            )
            .route(
                "/api/v1/events/internal",
                post(handlers::post_events_internal),
            )
            .with_state(state)
            .layer(middleware::from_fn(print_request_response));

        let server = Server::try_bind(address)?.serve(app.into_make_service());
        tracing::info!("listening on {address}", address = server.local_addr());
        Ok(AppServer {
            server,
            shutdown_rx,
        })
    }

    pub async fn run(mut self) -> Result<()> {
        self.server
            .with_graceful_shutdown(self.shutdown_rx.wait())
            .await?;
        Ok(())
    }
}
