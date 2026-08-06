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
        name_en: None,
        country_code: None,
        country_flag: None,
        xray: None,
        hysteria: None,
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

#[tokio::test]
async fn test_traffic_repository_and_command_validation() {
    dotenvy::dotenv().ok();
    let _ = tracing_subscriber::fmt::try_init();

    let config = Config::from_env().unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .unwrap();
    run_database_migrations(&pool).await.unwrap();

    let state = build_app_state(pool.clone(), config.clone());
    let add_traffic_cmd = state.traffic.add_traffic.clone();
    
    let fake_user_id = Uuid::new_v4().to_string();
    
    // 1. Get summary for non-existent user should be 0 total and 0 remaining
    let publisher = std::sync::Arc::new(hub::common::app::adapters::HttpCentrifugoClient::new(config.clone()));
    let traffic_repo = hub::features::traffic::PgTrafficRepository::new(pool.clone(), publisher);
    use hub::features::traffic::TrafficRepository;
    let summary = traffic_repo.get_summary(&fake_user_id).await.unwrap();
    assert_eq!(summary.total_bytes, 0);
    assert_eq!(summary.remaining_bytes, 0);

    // 2. Execute AddTrafficCommand with invalid <= 0 bytes should return ValidationError
    let err_res = add_traffic_cmd.execute(&fake_user_id, 0).await;
    assert!(err_res.is_err());
    assert!(err_res.unwrap_err().to_string().contains("greater than zero"));

    let err_res_neg = add_traffic_cmd.execute(&fake_user_id, -100).await;
    assert!(err_res_neg.is_err());
    assert!(err_res_neg.unwrap_err().to_string().contains("greater than zero"));
}

#[tokio::test]
async fn test_traffic_centrifuge_publishing() {
    dotenvy::dotenv().ok();
    let _ = tracing_subscriber::fmt::try_init();

    let config = Config::from_env().unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .unwrap();
    run_database_migrations(&pool).await.unwrap();

    // 1. Create a dummy user
    let user_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO users (id, username) VALUES ($1::uuid, $2)")
        .bind(Uuid::parse_str(&user_id).unwrap())
        .bind(format!("centrifuge-user-{}", Uuid::new_v4()))
        .execute(&pool)
        .await
        .unwrap();

    // 2. Setup mock publisher
    use hub::common::http::error::AppError;
    use hub::features::traffic::TrafficRepository;
    use hub::features::traffic::application::ports::RealtimePublisher;
    use std::sync::Mutex;
    struct MockPublisher {
        publications: Mutex<Vec<(String, serde_json::Value)>>,
    }
    #[async_trait::async_trait]
    impl RealtimePublisher for MockPublisher {
        async fn publish(&self, channel: &str, payload: serde_json::Value) -> Result<(), AppError> {
            self.publications.lock().unwrap().push((channel.to_string(), payload));
            Ok(())
        }
    }

    let mock_pub = std::sync::Arc::new(MockPublisher {
        publications: Mutex::new(Vec::new()),
    });

    // 3. Create repository with mock publisher
    let traffic_repo = hub::features::traffic::PgTrafficRepository::new(pool.clone(), mock_pub.clone());

    // 4. Add traffic packet (10 GB = 10737418240 bytes)
    let ten_gb = 10 * 1024 * 1024 * 1024;
    let _packet = traffic_repo.add_packet(&user_id, ten_gb).await.unwrap();

    // Verify publication triggered by add_packet
    {
        let pubs = mock_pub.publications.lock().unwrap();
        assert_eq!(pubs.len(), 1);
        let (ref channel, ref payload) = pubs[0];
        assert_eq!(channel, &format!("personal:{}", user_id));
        assert_eq!(payload.get("traffic_total_bytes").unwrap().as_i64().unwrap(), ten_gb);
        assert_eq!(payload.get("traffic_remaining_bytes").unwrap().as_i64().unwrap(), ten_gb);
    }

    // 5. Consume traffic (2 GB = 2147483648 bytes)
    let two_gb: u64 = 2 * 1024 * 1024 * 1024;
    let remaining = traffic_repo.consume(&user_id, two_gb).await.unwrap();
    assert_eq!(remaining, (ten_gb - two_gb as i64) as u64);

    // Verify publication triggered by consume
    {
        let pubs = mock_pub.publications.lock().unwrap();
        assert_eq!(pubs.len(), 2);
        let (ref channel, ref payload) = pubs[1];
        assert_eq!(channel, &format!("personal:{}", user_id));
        assert_eq!(payload.get("traffic_total_bytes").unwrap().as_i64().unwrap(), ten_gb);
        assert_eq!(payload.get("traffic_remaining_bytes").unwrap().as_i64().unwrap(), (ten_gb - two_gb as i64));
    }
}

#[tokio::test]
async fn test_real_centrifugo_integration() {
    dotenvy::dotenv().ok();
    let _ = tracing_subscriber::fmt::try_init();

    let config = Config::from_env().unwrap();
    let jwt_secret = std::env::var("JWT_SECRET_KEY").unwrap_or_default();
    println!("TEST JWT_SECRET_KEY: {}", jwt_secret);
    
    // Check if Centrifugo is running locally at port 38000
    let centrifugo_addr = "127.0.0.1:38000";
    if tokio::net::TcpStream::connect(centrifugo_addr).await.is_err() {
        println!("Centrifugo is not running at {}, skipping real integration test.", centrifugo_addr);
        return;
    }

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .unwrap();
    run_database_migrations(&pool).await.unwrap();

    // 1. Create a dummy user
    let user_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO users (id, username) VALUES ($1::uuid, $2)")
        .bind(Uuid::parse_str(&user_id).unwrap())
        .bind(format!("centrifuge-real-{}", Uuid::new_v4()))
        .execute(&pool)
        .await
        .unwrap();

    // 2. Generate tokens using JWT_SECRET_KEY
    let connection_token = hub::common::security::jwt::make_centrifugo_connect_token(&user_id).unwrap();
    let channel = format!("personal:{}", user_id);
    let subscription_token = hub::common::security::jwt::make_centrifugo_subscribe_token(&user_id, &channel).unwrap();

    // 3. Connect to Centrifugo WebSocket
    let ws_url = "ws://127.0.0.1:38000/connection/websocket";
    let (ws_stream, _) = tokio_tungstenite::connect_async(ws_url).await.unwrap();
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // 4. Send Connect message
    let connect_msg = serde_json::json!({
        "connect": {
            "token": connection_token
        },
        "id": 1
    });
    ws_sender
        .send(tokio_tungstenite::tungstenite::Message::Text(
            serde_json::to_string(&connect_msg).unwrap().into(),
        ))
        .await
        .unwrap();

    // Read Connect reply
    let msg = ws_receiver.next().await.unwrap().unwrap();
    let msg_text = msg.to_text().unwrap();
    println!("Connect reply: {}", msg_text);
    assert!(msg_text.contains("\"connect\"") && msg_text.contains("\"id\":1"));

    // 5. Send Subscribe message
    let subscribe_msg = serde_json::json!({
        "subscribe": {
            "channel": channel,
            "token": subscription_token
        },
        "id": 2
    });
    ws_sender
        .send(tokio_tungstenite::tungstenite::Message::Text(
            serde_json::to_string(&subscribe_msg).unwrap().into(),
        ))
        .await
        .unwrap();

    // Read Subscribe reply
    let msg = ws_receiver.next().await.unwrap().unwrap();
    let msg_text = msg.to_text().unwrap();
    println!("Subscribe reply: {}", msg_text);
    assert!(msg_text.contains("\"subscribe\"") && msg_text.contains("\"id\":2"));

    // 6. Setup client and publish event via HTTP API
    let centrifugo_client = hub::common::app::adapters::HttpCentrifugoClient::new(config);
    use hub::features::traffic::application::ports::RealtimePublisher;
    let payload = serde_json::json!({
        "traffic_total_bytes": 1000,
        "traffic_remaining_bytes": 800
    });
    centrifugo_client.publish(&channel, payload).await.unwrap();

    // 7. Receive Push message over WS
    let msg = ws_receiver.next().await.unwrap().unwrap();
    let msg_text = msg.to_text().unwrap();
    println!("Received push message: {}", msg_text);
    assert!(msg_text.contains("\"push\""));
    assert!(msg_text.contains("1000"));
    assert!(msg_text.contains("800"));
}

#[tokio::test]
async fn test_subtract_and_set_traffic_commands() {
    dotenvy::dotenv().ok();
    let _ = tracing_subscriber::fmt::try_init();

    let config = Config::from_env().unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .unwrap();
    run_database_migrations(&pool).await.unwrap();

    let state = build_app_state(pool.clone(), config.clone());
    let user_id = Uuid::new_v4().to_string();

    // Insert test user
    sqlx::query("INSERT INTO users (id, username) VALUES ($1::uuid, $2)")
        .bind(Uuid::parse_str(&user_id).unwrap())
        .bind(format!("sub-set-user-{}", Uuid::new_v4()))
        .execute(&pool)
        .await
        .unwrap();

    // 1. Add 1000 MB
    let one_thousand_mb_bytes = 1000 * 1024 * 1024;
    state.traffic.add_traffic.execute(&user_id, one_thousand_mb_bytes).await.unwrap();

    let summary = state.traffic.get_summary.execute(&user_id).await.unwrap();
    assert_eq!(summary.remaining_bytes, one_thousand_mb_bytes);

    // 2. Subtract 400 MB
    let four_hundred_mb_bytes = 400 * 1024 * 1024;
    let summary_after_sub = state.traffic.subtract_traffic.execute(&user_id, four_hundred_mb_bytes).await.unwrap();
    assert_eq!(summary_after_sub.remaining_bytes, (1000 - 400) * 1024 * 1024);

    // 3. Subtract 0 bytes should return error
    let err_sub = state.traffic.subtract_traffic.execute(&user_id, 0).await;
    assert!(err_sub.is_err());

    // 4. Set traffic to 2000 MB (increase)
    let two_thousand_mb_bytes = 2000 * 1024 * 1024;
    let summary_after_set_inc = state.traffic.set_traffic.execute(&user_id, two_thousand_mb_bytes).await.unwrap();
    assert_eq!(summary_after_set_inc.remaining_bytes, two_thousand_mb_bytes as i64);

    // 5. Set traffic to 500 MB (decrease)
    let five_hundred_mb_bytes = 500 * 1024 * 1024;
    let summary_after_set_dec = state.traffic.set_traffic.execute(&user_id, five_hundred_mb_bytes).await.unwrap();
    assert_eq!(summary_after_set_dec.remaining_bytes, five_hundred_mb_bytes as i64);
}




