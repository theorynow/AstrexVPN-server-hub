use futures_util::{SinkExt, StreamExt};
use hub::{
    app::create_router,
    common::app::{
        bootstrap::{build_app_state, run_database_migrations},
        config::Config,
    },
    features::nodes::{
        api::dto::{HubMessage, NodeMessage},
        domain::model::NodeStatus,
    },
};
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

#[tokio::test]
async fn test_nodes_websocket_lifecycle() {
    dotenvy::dotenv().ok();
    let _ = tracing_subscriber::fmt::try_init();

    // 1. Load configuration and connect to the test DB
    let config = Config::from_env().unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .min_connections(1)
        .connect(&config.database_url)
        .await
        .unwrap();
    run_database_migrations(&pool).await.unwrap();

    // 2. Start the application router on an ephemeral port
    let app_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let app_addr = app_listener.local_addr().unwrap();
    let state = build_app_state(pool.clone(), config.clone());
    let node_commander = state.nodes.node_commander.clone();
    let node_repo = state.nodes.node_repository.clone();

    let app = create_router(state);
    tokio::spawn(async move {
        axum::serve(app_listener, app).await.unwrap();
    });

    let ws_url = format!("ws://{}/ws/node", app_addr);

    // --- Case 1: Connect and Register successfully ---
    let node_id = format!("test-node-{}", Uuid::new_v4());
    let (ws_stream, _) = connect_async(&ws_url).await.unwrap();
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Send register message
    let reg_msg = NodeMessage::Register {
        node_id: node_id.clone(),
        auth_secret: config.node_auth_secret.clone(),
        public_ip: "127.0.0.1".to_string(),
        inbound_tags: vec!["vless-in".to_string()],
    };
    ws_sender
        .send(Message::Text(
            serde_json::to_string(&reg_msg).unwrap().into(),
        ))
        .await
        .unwrap();

    // Wait for AuthOk
    let msg = ws_receiver.next().await.unwrap().unwrap();
    match msg {
        Message::Text(text) => {
            let hub_msg: HubMessage = serde_json::from_str(&text).unwrap();
            match hub_msg {
                HubMessage::AuthOk => {}
                other => panic!("Expected AuthOk, got {:?}", other),
            }
        }
        other => panic!("Expected Text message, got {:?}", other),
    }

    // Verify node is online in database
    let node = node_repo.find_by_id(&node_id).await.unwrap().unwrap();
    assert_eq!(node.status, NodeStatus::Online);

    // --- Case 2: Send Ping and receive Pong ---
    ws_sender
        .send(Message::Text(
            serde_json::to_string(&NodeMessage::Ping).unwrap().into(),
        ))
        .await
        .unwrap();
    let msg = ws_receiver.next().await.unwrap().unwrap();
    match msg {
        Message::Text(text) => {
            let hub_msg: HubMessage = serde_json::from_str(&text).unwrap();
            match hub_msg {
                HubMessage::Pong => {}
                other => panic!("Expected Pong, got {:?}", other),
            }
        }
        other => panic!("Expected Text message, got {:?}", other),
    }

    // --- Case 3: Execute Command from Hub and reply with CommandResult ---
    let node_id_clone = node_id.clone();
    let commander_clone = node_commander.clone();
    let cmd_handle = tokio::spawn(async move {
        commander_clone
            .execute_add_user(&node_id_clone, "user-uuid-123")
            .await
    });

    // Receive AddUser command on client
    let msg = ws_receiver.next().await.unwrap().unwrap();
    let command_id = match msg {
        Message::Text(text) => {
            let hub_msg: HubMessage = serde_json::from_str(&text).unwrap();
            match hub_msg {
                HubMessage::AddUser {
                    command_id,
                    uuid,
                    inbound_tags,
                } => {
                    assert_eq!(uuid, "user-uuid-123");
                    assert_eq!(inbound_tags, vec!["vless-in".to_string()]);
                    command_id
                }
                other => panic!("Expected AddUser, got {:?}", other),
            }
        }
        other => panic!("Expected Text message, got {:?}", other),
    };

    // Send back CommandResult
    let result_msg = NodeMessage::CommandResult {
        command_id,
        success: true,
        error_message: String::new(),
    };
    ws_sender
        .send(Message::Text(
            serde_json::to_string(&result_msg).unwrap().into(),
        ))
        .await
        .unwrap();

    // Assert that the command execution completed successfully on the Hub side
    let cmd_res = cmd_handle.await.unwrap();
    assert!(cmd_res.is_ok());
    assert!(cmd_res.unwrap());

    // --- Case 4: Client disconnects and Node is marked Offline ---
    drop(ws_sender);
    drop(ws_receiver);

    // Give a short time for the connection clean up to run
    tokio::time::sleep(Duration::from_millis(150)).await;

    let node = node_repo.find_by_id(&node_id).await.unwrap().unwrap();
    assert_eq!(node.status, NodeStatus::Offline);

    // --- Case 5: Registration fails with wrong secret ---
    let (ws_stream, _) = connect_async(&ws_url).await.unwrap();
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    let reg_msg_bad = NodeMessage::Register {
        node_id: format!("test-node-bad-{}", Uuid::new_v4()),
        auth_secret: "wrong-secret".to_string(),
        public_ip: "127.0.0.1".to_string(),
        inbound_tags: vec![],
    };
    ws_sender
        .send(Message::Text(
            serde_json::to_string(&reg_msg_bad).unwrap().into(),
        ))
        .await
        .unwrap();

    // Expect AuthFailed
    let msg = ws_receiver.next().await.unwrap().unwrap();
    match msg {
        Message::Text(text) => {
            let hub_msg: HubMessage = serde_json::from_str(&text).unwrap();
            match hub_msg {
                HubMessage::AuthFailed { reason } => {
                    assert!(
                        reason.contains("WrongCredentials")
                            || reason.contains("Wrong credentials")
                            || reason.contains("authentication failed")
                    );
                }
                other => panic!("Expected AuthFailed, got {:?}", other),
            }
        }
        other => panic!("Expected Text message, got {:?}", other),
    }

    // Verify connection closed by server (cleanly or via reset)
    match ws_receiver.next().await {
        None => {}
        Some(Err(_)) => {}
        Some(Ok(Message::Close(_))) => {}
        Some(Ok(msg)) => panic!("Expected connection closure, got message: {:?}", msg),
    }
}

#[tokio::test]
async fn test_nodes_websocket_edge_cases() {
    dotenvy::dotenv().ok();
    let _ = tracing_subscriber::fmt::try_init();

    let config = Config::from_env().unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .unwrap();
    run_database_migrations(&pool).await.unwrap();

    let app_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let app_addr = app_listener.local_addr().unwrap();
    let state = build_app_state(pool, config);
    let app = create_router(state);
    tokio::spawn(async move {
        axum::serve(app_listener, app).await.unwrap();
    });

    let ws_url = format!("ws://{}/ws/node", app_addr);

    // --- Case 1: First message is NOT Register (e.g. NodeMessage::Ping) ---
    let (ws_stream1, _) = connect_async(&ws_url).await.unwrap();
    let (mut sender1, mut receiver1) = ws_stream1.split();
    
    sender1
        .send(Message::Text(serde_json::to_string(&NodeMessage::Ping).unwrap().into()))
        .await
        .unwrap();
    
    // Server should close connection without sending anything or closing cleanly
    match receiver1.next().await {
        None => {}
        Some(Err(_)) => {}
        Some(Ok(Message::Close(_))) => {}
        Some(Ok(msg)) => panic!("Expected connection closure, got: {:?}", msg),
    }

    // --- Case 2: First message is unparseable JSON ---
    let (ws_stream2, _) = connect_async(&ws_url).await.unwrap();
    let (mut sender2, mut receiver2) = ws_stream2.split();
    
    sender2
        .send(Message::Text("invalid json string here".into()))
        .await
        .unwrap();
    
    match receiver2.next().await {
        None => {}
        Some(Err(_)) => {}
        Some(Ok(Message::Close(_))) => {}
        Some(Ok(msg)) => panic!("Expected connection closure for invalid JSON, got: {:?}", msg),
    }

    // --- Case 3: Empty node_id in Register message ---
    let (ws_stream3, _) = connect_async(&ws_url).await.unwrap();
    let (mut sender3, mut receiver3) = ws_stream3.split();
    
    let bad_reg = NodeMessage::Register {
        node_id: "".to_string(),
        auth_secret: "secret".to_string(),
        public_ip: "127.0.0.1".to_string(),
        inbound_tags: vec![],
    };
    sender3
        .send(Message::Text(serde_json::to_string(&bad_reg).unwrap().into()))
        .await
        .unwrap();

    let msg = receiver3.next().await.unwrap().unwrap();
    match msg {
        Message::Text(text) => {
            let hub_msg: HubMessage = serde_json::from_str(&text).unwrap();
            match hub_msg {
                HubMessage::AuthFailed { reason } => {
                    assert!(reason.contains("empty") || reason.contains("validation"));
                }
                other => panic!("Expected AuthFailed for empty fields, got {:?}", other),
            }
        }
        other => panic!("Expected Text message, got {:?}", other),
    }
}

