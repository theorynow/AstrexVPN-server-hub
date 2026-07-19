use axum::{
    routing::{get, post},
    Router,
};

use crate::{
    common::app::state::AppState,
    features::nodes::api::handlers::{
        http_routes::{add_user_to_node, get_active_nodes, remove_user_from_node},
        ws_hub::ws_handler,
    },
};

pub fn node_routes() -> Router<AppState> {
    Router::new()
        .route("/active", get(get_active_nodes))
        .route("/{node_id}/users/{user_uuid}", post(add_user_to_node))
        .route(
            "/{node_id}/users/{user_uuid}",
            axum::routing::delete(remove_user_from_node),
        )
}

pub fn ws_routes() -> Router<AppState> {
    Router::new().route("/ws/node", get(ws_handler))
}

pub use crate::features::nodes::api::handlers::http_routes::NodesApiDoc;
