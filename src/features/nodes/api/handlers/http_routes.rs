use axum::{
    extract::{Path, State},
    Json,
};
use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    OpenApi,
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

#[derive(OpenApi)]
#[openapi(
    paths(
        get_active_nodes,
        add_user_to_node,
        remove_user_from_node,
    ),
    components(schemas(NodeDto)),
    tags(
        (name = "Nodes", description = "VPN Node management endpoints. Note: WebSocket agent connection is at /ws/node")
    ),
    modifiers(&NodesApiDoc)
)]
pub struct NodesApiDoc;

impl utoipa::Modify for NodesApiDoc {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap();
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .description(Some("Input your `<your-jwt>`"))
                    .build(),
            ),
        )
    }
}

#[utoipa::path(
    get,
    path = "/nodes/active",
    responses((status = 200, description = "Get list of active nodes", body = [NodeDto])),
    tag = "Nodes",
    security(("bearer_auth" = []))
)]
pub(crate) async fn get_active_nodes(
    State(state): State<crate::common::app::state::AppState>,
) -> Result<Json<ApiResponse<Vec<NodeDto>>>, AppError> {
    let query = GetActiveNodesQuery::new(state.nodes.node_repository.clone());
    let nodes = query.execute().await?;
    let dtos = nodes.into_iter().map(NodeDto::from).collect();
    Ok(Json(ApiResponse::success(dtos)))
}

#[utoipa::path(
    post,
    path = "/nodes/{node_id}/users/{user_uuid}",
    params(
        ("node_id" = String, Path, description = "Node ID"),
        ("user_uuid" = String, Path, description = "User UUID")
    ),
    responses((status = 200, description = "User added to node successfully")),
    tag = "Nodes",
    security(("bearer_auth" = []))
)]
pub(crate) async fn add_user_to_node(
    State(state): State<crate::common::app::state::AppState>,
    Path((node_id, user_uuid)): Path<(String, String)>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let cmd = AddUserToNodeCommand::new(state.nodes.node_commander.clone());
    cmd.execute(&node_id, &user_uuid).await?;
    Ok(Json(ApiResponse::success(())))
}

#[utoipa::path(
    delete,
    path = "/nodes/{node_id}/users/{user_uuid}",
    params(
        ("node_id" = String, Path, description = "Node ID"),
        ("user_uuid" = String, Path, description = "User UUID")
    ),
    responses((status = 200, description = "User removed from node successfully")),
    tag = "Nodes",
    security(("bearer_auth" = []))
)]
pub(crate) async fn remove_user_from_node(
    State(state): State<crate::common::app::state::AppState>,
    Path((node_id, user_uuid)): Path<(String, String)>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let cmd = RemoveUserFromNodeCommand::new(state.nodes.node_commander.clone());
    cmd.execute(&node_id, &user_uuid).await?;
    Ok(Json(ApiResponse::success(())))
}
