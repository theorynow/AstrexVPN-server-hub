use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::features::nodes::{
    api::dto::{HubMessage, NodeMessage},
    application::commands::{
        connect_node::ConnectNodeCommand, report_traffic::ReportTrafficCommand,
    },
    domain::model::NodeStatus,
};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<crate::common::app::state::AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: crate::common::app::state::AppState) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // 1. Read first message to authenticate connection
    let first_msg = loop {
        match ws_receiver.next().await {
            Some(Ok(Message::Text(text))) => match serde_json::from_str::<NodeMessage>(&text) {
                Ok(msg) => break msg,
                Err(e) => {
                    tracing::warn!(error = %e, "Invalid JSON in first message");
                    return;
                }
            },
            Some(Ok(Message::Ping(_))) => {}
            Some(Ok(Message::Pong(_))) => {}
            Some(Ok(Message::Close(_))) => {
                tracing::warn!("Connection closed by client during auth");
                return;
            }
            Some(Ok(_)) => {}
            Some(Err(e)) => {
                tracing::warn!(error = %e, "Error reading first message");
                return;
            }
            None => {
                tracing::warn!("Connection closed before receiving first message");
                return;
            }
        }
    };

    // First message must be NodeMessage::Register
    let (node_id, auth_secret, public_ip, inbound_tags, name_en, country_code, country_flag, xray, hysteria) = match first_msg {
        NodeMessage::Register {
            node_id,
            auth_secret,
            public_ip,
            inbound_tags,
            name_en,
            country_code,
            country_flag,
            xray,
            hysteria,
        } => {
            let name_en = name_en.filter(|s| !s.is_empty()).unwrap_or_else(|| format!("Server {}", node_id));
            let country_code = country_code.filter(|s| !s.is_empty()).unwrap_or_else(|| "DE".to_string());
            let country_flag = country_flag.filter(|s| !s.is_empty()).unwrap_or_else(|| "🌐".to_string());
            (node_id, auth_secret, public_ip, inbound_tags, name_en, country_code, country_flag, xray, hysteria)
        }
        _ => {
            tracing::warn!("First message was not a Register message");
            return;
        }
    };

    if node_id.is_empty() || auth_secret.is_empty() {
        tracing::warn!("Node ID or auth secret is empty");
        let response = HubMessage::AuthFailed {
            reason: "Node ID or auth secret is empty".to_string(),
        };
        if let Ok(res_str) = serde_json::to_string(&response) {
            let _ = ws_sender.send(Message::Text(res_str.into())).await;
        }
        return;
    }

    // Validate the connection through our business logic
    let connect_node_cmd = ConnectNodeCommand::new(
        state.nodes.node_repository.clone(),
        state.config.node_auth_secret.clone(),
    );

    if let Err(e) = connect_node_cmd
        .execute(&node_id, &auth_secret, &public_ip, &name_en, &country_code, &country_flag, xray, hysteria)
        .await
    {
        tracing::warn!(node_id = %node_id, error = %e, "Node authentication failed");
        let response = HubMessage::AuthFailed {
            reason: e.to_string(),
        };
        if let Ok(res_str) = serde_json::to_string(&response) {
            let _ = ws_sender.send(Message::Text(res_str.into())).await;
        }
        return;
    }

    // Auth OK
    let auth_ok_msg = HubMessage::AuthOk;
    if let Ok(auth_ok_str) = serde_json::to_string(&auth_ok_msg) {
        if ws_sender
            .send(Message::Text(auth_ok_str.into()))
            .await
            .is_err()
        {
            return;
        }
    }

    // 2. Create the bidirectional transmission channel
    let (tx, mut rx) = mpsc::channel::<HubMessage>(100);

    // Register the sender and inbound tags inside the WsCommanderImpl adapter
    // Note: state.nodes.grpc_commander is our Arc<dyn NodeCommander>
    state
        .nodes
        .node_commander
        .register_node(node_id.clone(), tx, inbound_tags);

    // Mark online in DB immediately
    let repo = state.nodes.node_repository.clone();
    let _ = repo.update_status(&node_id, NodeStatus::Online).await;

    tracing::info!(node_id = %node_id, public_ip = %public_ip, "Node WebSocket connected successfully");

    let report_traffic_cmd = Arc::new(ReportTrafficCommand::new(
        state.nodes.user_traffic_service.clone(),
        state.nodes.node_commander.clone(),
    ));
    let node_id_clone = node_id.clone();
    let node_commander = state.nodes.node_commander.clone();

    let mut ping_interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
    // Reset interval timer so it does not tick immediately
    ping_interval.tick().await;

    loop {
        tokio::select! {
            // Outbound: from local mpsc channel to WebSocket
            msg = rx.recv() => {
                match msg {
                    Some(hub_msg) => {
                        match serde_json::to_string(&hub_msg) {
                            Ok(json_str) => {
                                if ws_sender.send(Message::Text(json_str.into())).await.is_err() {
                                    tracing::error!(node_id = %node_id_clone, "Failed to send message to node, closing connection");
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::error!(node_id = %node_id_clone, error = %e, "Failed to serialize HubMessage");
                            }
                        }
                    }
                    None => {
                        tracing::info!(node_id = %node_id_clone, "Outbound channel closed");
                        break;
                    }
                }
            }
            // Ping interval: send a Pong (keepalive) to detect dead connections
            _ = ping_interval.tick() => {
                let ping_msg = HubMessage::Pong;
                match serde_json::to_string(&ping_msg) {
                    Ok(json_str) => {
                        if ws_sender.send(Message::Text(json_str.into())).await.is_err() {
                            tracing::error!(node_id = %node_id_clone, "Ping interval send failed, closing connection");
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::error!(node_id = %node_id_clone, error = %e, "Failed to serialize Pong keepalive");
                    }
                }
            }
            // Inbound: from WebSocket to local handlers
            msg = ws_receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<NodeMessage>(&text) {
                            Ok(node_msg) => {
                                match node_msg {
                                    NodeMessage::Ping => {
                                        // Respond with Pong immediately
                                        let pong = HubMessage::Pong;
                                        if let Ok(pong_str) = serde_json::to_string(&pong) {
                                            if ws_sender.send(Message::Text(pong_str.into())).await.is_err() {
                                                tracing::error!(node_id = %node_id_clone, "Failed to send Pong to Ping");
                                                break;
                                            }
                                        }
                                    }
                                    NodeMessage::TrafficReport { user_bytes } => {
                                         let cmd = report_traffic_cmd.clone();
                                         let nid = node_id_clone.clone();
                                         tokio::spawn(async move {
                                             cmd.execute(&nid, user_bytes).await;
                                         });
                                    }
                                    NodeMessage::CommandResult { command_id, success, error_message } => {
                                        let res = crate::features::nodes::domain::ports::node_commander::CommandResult {
                                            success,
                                            error_message,
                                        };
                                        node_commander.resolve_command(&command_id, res);
                                    }
                                    _ => {
                                        // Register is ignored after connection is established
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!(node_id = %node_id_clone, error = %e, "Failed to parse NodeMessage");
                            }
                        }
                    }
                    Some(Ok(_)) => {
                        // Ignore non-text messages
                    }
                    Some(Err(e)) => {
                        tracing::error!(node_id = %node_id_clone, error = %e, "Error reading from WebSocket");
                        break;
                    }
                    None => {
                        tracing::info!(node_id = %node_id_clone, "WebSocket connection closed by node");
                        break;
                    }
                }
            }
        }
    }

    // Cleanup when WebSocket closes
    tracing::warn!(node_id = %node_id_clone, "Node WebSocket disconnected");
    node_commander.deregister_node(&node_id_clone);
    let _ = repo
        .update_status(&node_id_clone, NodeStatus::Offline)
        .await;
}
