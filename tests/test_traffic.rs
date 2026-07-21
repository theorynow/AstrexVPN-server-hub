use futures_util::{SinkExt, StreamExt};
use hub::{
    app::create_router,
    common::app::{
        bootstrap::{build_app_state, run_database_migrations},
        config::Config,
    },
    features::nodes::{
        api::dto::{HubMessage, NodeMessage},
    },
};
use std::collections::HashMap;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

#[tokio::test]
async fn test_traffic_packets_lifecycle() {
    dotenvy::dotenv().ok();
    let _ = tracing_subscriber::fmt::try_init();

    // 1. Load configuration and connect to the test DB
    let config = Config::from_env().unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .unwrap();
    run_database_migrations(&pool).await.unwrap();

    // 2. Start the application router on an ephemeral port
    let app_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let app_addr = app_listener.local_addr().unwrap();
    let state = build_app_state(pool.clone(), config.clone());
    let node_commander = state.nodes.node_commander.clone();
    
    let app = create_router(state.clone());
    tokio::spawn(async move {
        axum::serve(app_listener, app).await.unwrap();
    });

    let ws_url = format!("ws://{}/ws/node", app_addr);

    // --- Prepare test user ---
    let user_id = Uuid::new_v4();
    // Insert user manually into test database
    sqlx::query("INSERT INTO users (id, username) VALUES ($1, $2)")
        .bind(user_id)
        .bind(format!("traffic-user-{}", user_id))
        .execute(&pool)
        .await
        .unwrap();

    // Insert traffic packets:
    // Packet A: 1000 bytes remaining, expires in 1 day
    let packet_a_id = Uuid::new_v4();
    sqlx::query("INSERT INTO user_traffic_packets (id, user_id, traffic_limit_bytes, traffic_remaining_bytes, expires_at) VALUES ($1, $2, 1000, 1000, now() + interval '1 day')")
        .bind(packet_a_id)
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();

    // Packet B: 500 bytes remaining, expires in 2 days
    let packet_b_id = Uuid::new_v4();
    sqlx::query("INSERT INTO user_traffic_packets (id, user_id, traffic_limit_bytes, traffic_remaining_bytes, expires_at) VALUES ($1, $2, 500, 500, now() + interval '2 days')")
        .bind(packet_b_id)
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();

    // Verify initial remaining traffic (1500 bytes)
    let traffic_service = state.nodes.user_traffic_service.clone();
    let remaining = traffic_service.get_remaining_traffic(&user_id.to_string()).await.unwrap();
    assert_eq!(remaining, 1500);

    // --- Test Node WS connection and Commands ---
    let node_id = format!("test-node-traffic-{}", Uuid::new_v4());
    let (ws_stream, _) = connect_async(&ws_url).await.unwrap();
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Register node
    let reg_msg = NodeMessage::Register {
        node_id: node_id.clone(),
        auth_secret: config.node_auth_secret.clone(),
        public_ip: "127.0.0.1".to_string(),
        inbound_tags: vec!["vless-in".to_string()],
    };
    ws_sender
        .send(Message::Text(serde_json::to_string(&reg_msg).unwrap().into()))
        .await
        .unwrap();

    // Wait for AuthOk
    let msg = ws_receiver.next().await.unwrap().unwrap();
    assert!(matches!(
        serde_json::from_str::<HubMessage>(&msg.into_text().unwrap()).unwrap(),
        HubMessage::AuthOk
    ));

    // --- CASE A: Add user to node when they have remaining traffic ---
    let cmd_add_user = hub::features::nodes::application::commands::add_user_to_node::AddUserToNodeCommand::new(
        node_commander.clone(),
        traffic_service.clone(),
    );

    let cmd_handle = tokio::spawn({
        let node_id = node_id.clone();
        let user_uuid = user_id.to_string();
        async move {
            cmd_add_user.execute(&node_id, &user_uuid).await
        }
    });

    // Node client should receive AddUser command
    let msg = ws_receiver.next().await.unwrap().unwrap();
    let command_id = match serde_json::from_str::<HubMessage>(&msg.into_text().unwrap()).unwrap() {
        HubMessage::AddUser { command_id, uuid, .. } => {
            assert_eq!(uuid, user_id.to_string());
            command_id
        }
        other => panic!("Expected AddUser command, got {:?}", other),
    };

    // Node client replies with success
    let result_msg = NodeMessage::CommandResult {
        command_id,
        success: true,
        error_message: String::new(),
    };
    ws_sender
        .send(Message::Text(serde_json::to_string(&result_msg).unwrap().into()))
        .await
        .unwrap();

    let res = cmd_handle.await.unwrap();
    assert!(res.is_ok());

    // --- CASE B: Consume some traffic, verify packet sorting (smallest remaining first) ---
    // Total remaining: Packet B (500), Packet A (1000)
    // We consume 600 bytes. This should completely exhaust Packet B (500) and take 100 bytes from Packet A (1000 -> 900)
    let remaining_after = traffic_service.consume_traffic(&user_id.to_string(), 600).await.unwrap();
    assert_eq!(remaining_after, 900);

    // Verify Packet B remaining in DB is 0
    let pb_rem: i64 = sqlx::query_scalar("SELECT traffic_remaining_bytes FROM user_traffic_packets WHERE id = $1")
        .bind(packet_b_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(pb_rem, 0);

    // Verify Packet A remaining in DB is 900
    let pa_rem: i64 = sqlx::query_scalar("SELECT traffic_remaining_bytes FROM user_traffic_packets WHERE id = $1")
        .bind(packet_a_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(pa_rem, 900);

    // --- CASE C: Exhaust traffic, verify user gets removed from node ---
    // We send a TrafficReport via WebSocket from Node client. Let's report that user consumed 1000 bytes.
    // This exceeds the remaining 900 bytes, so remaining will drop to 0, and user should be removed.
    let report_msg = NodeMessage::TrafficReport {
        user_bytes: HashMap::from([(user_id.to_string(), 1000)]),
    };
    ws_sender
        .send(Message::Text(serde_json::to_string(&report_msg).unwrap().into()))
        .await
        .unwrap();

    // Node client should receive RemoveUser command automatically!
    let msg = ws_receiver.next().await.unwrap().unwrap();
    let command_id = match serde_json::from_str::<HubMessage>(&msg.into_text().unwrap()).unwrap() {
        HubMessage::RemoveUser { command_id, email, .. } => {
            assert_eq!(email, user_id.to_string());
            command_id
        }
        other => panic!("Expected RemoveUser command, got {:?}", other),
    };

    // Send back CommandResult
    let result_msg = NodeMessage::CommandResult {
        command_id,
        success: true,
        error_message: String::new(),
    };
    ws_sender
        .send(Message::Text(serde_json::to_string(&result_msg).unwrap().into()))
        .await
        .unwrap();

    // Traffic remaining should be 0
    let remaining_after_exhaustion = traffic_service.get_remaining_traffic(&user_id.to_string()).await.unwrap();
    assert_eq!(remaining_after_exhaustion, 0);

    // --- CASE D: Re-connecting user with 0 traffic fails ---
    let cmd_add_user_again = hub::features::nodes::application::commands::add_user_to_node::AddUserToNodeCommand::new(
        node_commander.clone(),
        traffic_service.clone(),
    );

    let res_fail = cmd_add_user_again.execute(&node_id, &user_id.to_string()).await;
    assert!(res_fail.is_err());
    let err_msg = res_fail.unwrap_err().to_string();
    assert!(err_msg.contains("no remaining traffic") || err_msg.contains("validation"));
}
