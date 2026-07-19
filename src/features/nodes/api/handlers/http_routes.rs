use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};

use crate::{
    common::http::{dto::ApiResponse, error::AppError},
    features::nodes::application::{
        commands::{
            add_user_to_node::AddUserToNodeCommand,
            remove_user_from_node::RemoveUserFromNodeCommand,
        },
        models::node_dto::NodeDto,
        queries::get_active_nodes::GetActiveNodesQuery,
    },
};

pub fn router(
    state: crate::common::app::state::AppState,
) -> Router<crate::common::app::state::AppState> {
    Router::new()
        .route("/active", get(get_active_nodes))
        .route("/{node_id}/users/{user_uuid}", post(add_user_to_node))
        .route(
            "/{node_id}/users/{user_uuid}",
            axum::routing::delete(remove_user_from_node),
        )
        .with_state(state)
}

async fn get_active_nodes(
    State(state): State<crate::common::app::state::AppState>,
) -> Result<Json<ApiResponse<Vec<NodeDto>>>, AppError> {
    let query = GetActiveNodesQuery::new(state.nodes.node_repository.clone());
    let nodes = query.execute().await?;
    let dtos = nodes.into_iter().map(NodeDto::from).collect();
    Ok(Json(ApiResponse::success(dtos)))
}

async fn add_user_to_node(
    State(state): State<crate::common::app::state::AppState>,
    Path((node_id, user_uuid)): Path<(String, String)>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let cmd = AddUserToNodeCommand::new(state.nodes.grpc_commander.clone());
    cmd.execute(&node_id, &user_uuid).await?;
    Ok(Json(ApiResponse::success(())))
}

async fn remove_user_from_node(
    State(state): State<crate::common::app::state::AppState>,
    Path((node_id, user_uuid)): Path<(String, String)>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let cmd = RemoveUserFromNodeCommand::new(state.nodes.grpc_commander.clone());
    cmd.execute(&node_id, &user_uuid).await?;
    Ok(Json(ApiResponse::success(())))
}
