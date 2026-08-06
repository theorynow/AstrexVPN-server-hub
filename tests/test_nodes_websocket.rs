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

use hub::features::nodes::domain::model::{HysteriaConfig, XrayConfig};

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

    // Send register message with full multilang & xray/hysteria configs
    let reg_msg = NodeMessage::Register {
        node_id: node_id.clone(),
        auth_secret: config.node_auth_secret.clone(),
        public_ip: "127.0.0.1".to_string(),
        inbound_tags: vec!["vless-in".to_string()],
        name_en: Some("Germany".to_string()),
        country_code: Some("DE".to_string()),
        country_flag: Some("🇩🇪".to_string()),
        xray: Some(XrayConfig {
            port: 443,
            sni: "www.yahoo.com".to_string(),
            public_key: "AW1VX2QqSTaHjqtnOR3j5SWStzqh5T3Ly7SjUzC_zU8".to_string(),
            short_id: "18ab3ba173244769".to_string(),
        }),
        hysteria: Some(HysteriaConfig {
            port: 443,
            sni: "fuckbook.pro".to_string(),
        }),
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

    // Verify node is online in database and has all fields
    let node = node_repo.find_by_id(&node_id).await.unwrap().unwrap();
    assert_eq!(node.status, NodeStatus::Online);
    assert_eq!(node.name_en, "Germany");
    assert_eq!(node.country_code, "DE");
    assert_eq!(node.country_flag, "🇩🇪");
    assert_eq!(
        node.xray,
        Some(XrayConfig {
            port: 443,
            sni: "www.yahoo.com".to_string(),
            public_key: "AW1VX2QqSTaHjqtnOR3j5SWStzqh5T3Ly7SjUzC_zU8".to_string(),
            short_id: "18ab3ba173244769".to_string(),
        })
    );
    assert_eq!(
        node.hysteria,
        Some(HysteriaConfig {
            port: 443,
            sni: "fuckbook.pro".to_string(),
        })
    );

    // Register & Login a test user to obtain JWT token for HTTP requests
    let http_client = reqwest::Client::new();
    let test_user_name = format!("testuser-{}", Uuid::new_v4());
    let _ = http_client
        .post(format!("http://{}/auth/register", app_addr))
        .json(&serde_json::json!({
            "username": test_user_name,
            "password": "password123"
        }))
        .send()
        .await
        .unwrap();

    let login_res = http_client
        .post(format!("http://{}/auth/login", app_addr))
        .json(&serde_json::json!({
            "username": test_user_name,
            "password": "password123"
        }))
        .send()
        .await
        .unwrap();
    let login_json: serde_json::Value = login_res.json().await.unwrap();
    let token = login_json["data"]["access_token"].as_str().unwrap();

    // Verify HTTP GET /nodes/active returns active nodes with name_en, name_ru, country_flag, xray, hysteria and NO status field
    let res = http_client
        .get(format!("http://{}/nodes/active", app_addr))
        .bearer_auth(token)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), reqwest::StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    let data = body.get("data").unwrap().as_array().unwrap();
    let active_node_json = data.iter().find(|n| n.get("id").unwrap().as_str() == Some(&node_id)).unwrap();

    assert_eq!(active_node_json.get("public_ip").unwrap().as_str().unwrap(), "127.0.0.1");
    assert_eq!(active_node_json.get("name_en").unwrap().as_str().unwrap(), "Germany");
    assert_eq!(active_node_json.get("name_ru").unwrap().as_str().unwrap(), "Германия");
    assert_eq!(active_node_json.get("country_flag").unwrap().as_str().unwrap(), "🇩🇪");
    assert!(active_node_json.get("status").is_none()); // status must be removed!

    let xray_json = active_node_json.get("xray").unwrap();
    assert_eq!(xray_json.get("port").unwrap().as_u64().unwrap(), 443);
    assert_eq!(xray_json.get("sni").unwrap().as_str().unwrap(), "www.yahoo.com");
    assert_eq!(xray_json.get("public_key").unwrap().as_str().unwrap(), "AW1VX2QqSTaHjqtnOR3j5SWStzqh5T3Ly7SjUzC_zU8");
    assert_eq!(xray_json.get("short_id").unwrap().as_str().unwrap(), "18ab3ba173244769");

    let hysteria_json = active_node_json.get("hysteria").unwrap();
    assert_eq!(hysteria_json.get("port").unwrap().as_u64().unwrap(), 443);
    assert_eq!(hysteria_json.get("sni").unwrap().as_str().unwrap(), "fuckbook.pro");

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
        name_en: None,
        name_ru: None,
        country_flag: None,
        xray: None,
        hysteria: None,
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
        name_en: None,
        name_ru: None,
        country_flag: None,
        xray: None,
        hysteria: None,
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

