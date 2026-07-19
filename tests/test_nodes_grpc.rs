use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tonic::Request;
use uuid::Uuid;

use hub::{
    app::create_router,
    common::{
        app::{
            bootstrap::{build_app_state, run_database_migrations},
            config::Config,
        },
        http::dto::RestApiResponse,
    },
    features::nodes::api::grpc_codegen::vpn::infrastructure::{
        hub_service_client::HubServiceClient, hub_service_server::HubServiceServer, inbound_config,
        node_message, CommandResult, InboundConfig, NodeMessage, NodeRegistration, RealitySettings,
        TrafficReport as ProtoTrafficReport,
    },
};

#[tokio::test]
async fn test_nodes_grpc_lifecycle() {
    dotenvy::dotenv().ok();
    let _ = tracing_subscriber::fmt::try_init();

    // 1. Load configuration and connect to the test DB
    let config = Config::from_env().unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .min_connections(1)
        .connect(&config.database_url)
        .await
        .unwrap();
    run_database_migrations(&pool).await.unwrap();

    // 2. Start the HTTP application router on an ephemeral port
    let app_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let app_addr = app_listener.local_addr().unwrap();
    let state = build_app_state(pool.clone(), config);
    let app = create_router(state.clone());
    tokio::spawn(async move {
        axum::serve(app_listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let base_url = format!("http://{}", app_addr);

    // Register and login a test admin/user to get JWT access token
    let username = format!("admin-{}", Uuid::new_v4());
    let reg_resp = client
        .post(format!("{}/auth/register", base_url))
        .json(&json!({
            "username": username,
            "password": "password123"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(reg_resp.status(), reqwest::StatusCode::OK);

    let login_resp = client
        .post(format!("{}/auth/login", base_url))
        .json(&json!({
            "username": username,
            "password": "password123"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(login_resp.status(), reqwest::StatusCode::OK);
    let login_body: RestApiResponse<Value> = login_resp.json().await.unwrap();
    let access_token = login_body
        .0
        .data
        .unwrap()
        .get("access_token")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();

    // 3. Find a free local port and start the Tonic gRPC server
    let grpc_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let grpc_addr = grpc_listener.local_addr().unwrap();
    drop(grpc_listener);

    let connect_node_cmd = std::sync::Arc::new(
        hub::features::nodes::application::commands::connect_node::ConnectNodeCommand::new(
            state.nodes.node_repository.clone(),
            state.config.node_auth_secret.clone(),
        ),
    );
    let report_traffic_cmd = std::sync::Arc::new(
        hub::features::nodes::application::commands::report_traffic::ReportTrafficCommand::new(),
    );
    let grpc_service = hub::features::nodes::api::handlers::grpc_hub::MyCoreHubService::new(
        connect_node_cmd,
        report_traffic_cmd,
        state.nodes.node_repository.clone(),
        state.nodes.grpc_commander.clone(),
    );

    tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(HubServiceServer::new(grpc_service))
            .serve(grpc_addr)
            .await
            .unwrap();
    });

    // Let the gRPC server start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // 4. Connect gRPC Client (representing the VPN Node)
    let mut grpc_client = HubServiceClient::connect(format!("http://{}", grpc_addr))
        .await
        .unwrap();

    let (tx, rx) = mpsc::channel(10);
    let node_id = "test-node-1".to_string();
    let auth_secret = "secret123".to_string();

    // Send the first message to authenticate the node connection BEFORE establishing the stream
    tx.send(NodeMessage {
        node_id: node_id.clone(),
        auth_secret: auth_secret.clone(),
        payload: Some(node_message::Payload::Registration(NodeRegistration {
            public_ip: "198.51.100.22".to_string(),
            inbounds: vec![InboundConfig {
                protocol: "vless".to_string(),
                port: 443,
                inbound_tag: "vless-reality-in".to_string(),
                settings: Some(inbound_config::Settings::Reality(RealitySettings {
                    pbk: "pbk123".to_string(),
                    sid: "sid123".to_string(),
                    sni: "sni123".to_string(),
                })),
            }],
        })),
    })
    .await
    .unwrap();

    let request_stream = ReceiverStream::new(rx);

    // Call the connect_node bidirectional streaming RPC
    let response = grpc_client
        .connect_node(Request::new(request_stream))
        .await
        .unwrap();
    let mut inbound_stream = response.into_inner();

    // Let the server authenticate and register the node
    tokio::time::sleep(Duration::from_millis(100)).await;

    // 5. Verify the node is listed as Active/Online via REST API
    let active_resp = client
        .get(format!("{}/nodes/active", base_url))
        .bearer_auth(&access_token)
        .send()
        .await
        .unwrap();
    assert_eq!(active_resp.status(), reqwest::StatusCode::OK);
    let active_body: RestApiResponse<Vec<Value>> = active_resp.json().await.unwrap();
    let active_nodes = active_body.0.data.unwrap();
    assert!(active_nodes
        .iter()
        .any(|node| node.get("id").unwrap().as_str().unwrap() == node_id));

    // 6. Spawn a task to emulate the node client handling incoming gRPC commands
    let tx_clone = tx.clone();
    let node_id_clone = node_id.clone();
    let auth_secret_clone = auth_secret.clone();
    tokio::spawn(async move {
        while let Some(Ok(cmd)) = inbound_stream.next().await {
            // Emulate successful command execution by returning success result
            tx_clone
                .send(NodeMessage {
                    node_id: node_id_clone.clone(),
                    auth_secret: auth_secret_clone.clone(),
                    payload: Some(node_message::Payload::CommandResult(CommandResult {
                        command_id: cmd.command_id,
                        success: true,
                        error_message: String::new(),
                    })),
                })
                .await
                .unwrap();
        }
    });

    // 7. Test Add User to Node command via REST API
    let user_uuid = Uuid::new_v4().to_string();
    let add_user_resp = client
        .post(format!(
            "{}/nodes/{}/users/{}",
            base_url, node_id, user_uuid
        ))
        .bearer_auth(&access_token)
        .send()
        .await
        .unwrap();
    assert_eq!(add_user_resp.status(), reqwest::StatusCode::OK);

    // 8. Test Remove User from Node command via REST API
    let remove_user_resp = client
        .delete(format!(
            "{}/nodes/{}/users/{}",
            base_url, node_id, user_uuid
        ))
        .bearer_auth(&access_token)
        .send()
        .await
        .unwrap();
    assert_eq!(remove_user_resp.status(), reqwest::StatusCode::OK);

    // 9. Test Reporting Traffic via gRPC stream
    let mut user_bytes = HashMap::new();
    user_bytes.insert(user_uuid, 5000000_u64); // 5 MB
    tx.send(NodeMessage {
        node_id: node_id.clone(),
        auth_secret: auth_secret.clone(),
        payload: Some(node_message::Payload::TrafficReport(ProtoTrafficReport {
            user_bytes,
        })),
    })
    .await
    .unwrap();

    // Give time for traffic report command execution to log output
    tokio::time::sleep(Duration::from_millis(50)).await;
}
