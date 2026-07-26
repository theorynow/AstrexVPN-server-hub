use axum::http::StatusCode;
use hub::{
    app::create_router,
    common::{
        app::{
            bootstrap::{build_app_state, run_database_migrations},
            config::Config,
        },
        http::dto::RestApiResponse,
    },
};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[tokio::test]
async fn test_user_routes_lifecycle() {
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

    // 2. Start the application router on an ephemeral port
    let app_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let app_addr = app_listener.local_addr().unwrap();
    let state = build_app_state(pool.clone(), config);
    let app = create_router(state);
    tokio::spawn(async move {
        axum::serve(app_listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let base_url = format!("http://{}", app_addr);

    // Register a test user
    let username1 = format!("u1-{}", Uuid::new_v4());
    let resp1 = client
        .post(format!("{}/auth/register", base_url))
        .json(&json!({
            "username": username1,
            "password": "password123"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp1.status(), StatusCode::OK);

    // Login to get access token
    let login_resp = client
        .post(format!("{}/auth/login", base_url))
        .json(&json!({
            "username": username1,
            "password": "password123"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(login_resp.status(), StatusCode::OK);
    let login_body: RestApiResponse<Value> = login_resp.json().await.unwrap();
    let access_token1 = login_body
        .0
        .data
        .unwrap()
        .get("access_token")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();

    // --- 1. Test GET /user/me ---
    let me_resp = client
        .get(format!("{}/user/me", base_url))
        .bearer_auth(&access_token1)
        .send()
        .await
        .unwrap();
    assert_eq!(me_resp.status(), StatusCode::OK);
    let me_body: RestApiResponse<Value> = me_resp.json().await.unwrap();
    let me_data = me_body.0.data.unwrap();
    let user_id1 = me_data.get("id").unwrap().as_str().unwrap().to_string();
    assert_eq!(
        me_data.get("username").unwrap().as_str().unwrap(),
        username1
    );

    // --- 2. Test PATCH /user/me ---
    let updated_username = format!("u1-new-{}", Uuid::new_v4());
    let patch_resp = client
        .patch(format!("{}/user/me", base_url))
        .bearer_auth(&access_token1)
        .json(&json!({
            "username": updated_username
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(patch_resp.status(), StatusCode::OK);
    let patch_body: RestApiResponse<Value> = patch_resp.json().await.unwrap();
    assert_eq!(
        patch_body
            .0
            .data
            .unwrap()
            .get("username")
            .unwrap()
            .as_str()
            .unwrap(),
        updated_username
    );

    // --- 2b. Test PATCH /user/me updating both username and password ---
    let updated_username2 = format!("u1-new2-{}", Uuid::new_v4());
    let new_password = "new_password_456";
    let patch_resp2 = client
        .patch(format!("{}/user/me", base_url))
        .bearer_auth(&access_token1)
        .json(&json!({
            "username": updated_username2,
            "password": new_password
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(patch_resp2.status(), StatusCode::OK);

    // Verify login with updated username and new password
    let login_new_resp = client
        .post(format!("{}/auth/login", base_url))
        .json(&json!({
            "username": updated_username2,
            "password": new_password
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(login_new_resp.status(), StatusCode::OK);

    // --- 2c. Test PATCH /user/me with duplicate username (should return CONFLICT 409) ---
    let username2 = format!("u2-{}", Uuid::new_v4());
    let reg2_resp = client
        .post(format!("{}/auth/register", base_url))
        .json(&json!({
            "username": username2,
            "password": "password123"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(reg2_resp.status(), StatusCode::OK);

    let dup_patch_resp = client
        .patch(format!("{}/user/me", base_url))
        .bearer_auth(&access_token1)
        .json(&json!({
            "username": username2
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(dup_patch_resp.status(), StatusCode::CONFLICT);

    // --- 3. Test GET /user/{id} ---
    let get_by_id_resp = client
        .get(format!("{}/user/{}", base_url, user_id1))
        .bearer_auth(&access_token1)
        .send()
        .await
        .unwrap();
    assert_eq!(get_by_id_resp.status(), StatusCode::OK);
    let get_by_id_body: RestApiResponse<Value> = get_by_id_resp.json().await.unwrap();
    assert_eq!(
        get_by_id_body
            .0
            .data
            .unwrap()
            .get("username")
            .unwrap()
            .as_str()
            .unwrap(),
        updated_username2
    );

    // --- 4. Test POST /traffic/add (in MB) ---
    let add_traffic_resp = client
        .post(format!("{}/traffic/add", base_url))
        .bearer_auth(&access_token1)
        .json(&json!({
            "user_id": user_id1,
            "mb": 10240
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(add_traffic_resp.status(), StatusCode::OK);
    let add_traffic_body: RestApiResponse<Value> = add_traffic_resp.json().await.unwrap();
    let traffic_limit_bytes = add_traffic_body
        .0
        .data
        .unwrap()
        .get("traffic_limit_bytes")
        .unwrap()
        .as_i64()
        .unwrap();
    assert_eq!(traffic_limit_bytes, 10737418240); // 10240 MB = 10 GB

    // Check GET /traffic/me to verify traffic total has increased to 35 GB (25 GB initial + 10 GB added)
    let traffic_resp = client
        .get(format!("{}/traffic/me", base_url))
        .bearer_auth(&access_token1)
        .send()
        .await
        .unwrap();
    assert_eq!(traffic_resp.status(), StatusCode::OK);
    let traffic_body: RestApiResponse<Value> = traffic_resp.json().await.unwrap();
    let traffic_data = traffic_body.0.data.unwrap();
    let total_bytes = traffic_data.get("traffic_total_bytes").unwrap().as_i64().unwrap();
    let remaining_bytes = traffic_data.get("traffic_remaining_bytes").unwrap().as_i64().unwrap();
    assert_eq!(total_bytes, 37580963840); // 35 GB
    assert_eq!(remaining_bytes, 37580963840); // 35 GB

    // --- 4b. Test POST /traffic/subtract (in MB) ---
    let sub_traffic_resp = client
        .post(format!("{}/traffic/subtract", base_url))
        .bearer_auth(&access_token1)
        .json(&json!({
            "user_id": user_id1,
            "mb": 5120 // Subtract 5 GB
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(sub_traffic_resp.status(), StatusCode::OK);
    let sub_traffic_body: RestApiResponse<Value> = sub_traffic_resp.json().await.unwrap();
    let remaining_after_sub = sub_traffic_body
        .0
        .data
        .unwrap()
        .get("traffic_remaining_bytes")
        .unwrap()
        .as_i64()
        .unwrap();
    assert_eq!(remaining_after_sub, 37580963840 - 5368709120); // 30 GB remaining

    // --- 4c. Test POST /traffic/set (in MB) ---
    let set_traffic_resp = client
        .post(format!("{}/traffic/set", base_url))
        .bearer_auth(&access_token1)
        .json(&json!({
            "user_id": user_id1,
            "mb": 20480 // Set remaining to 20 GB
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(set_traffic_resp.status(), StatusCode::OK);
    let set_traffic_body: RestApiResponse<Value> = set_traffic_resp.json().await.unwrap();
    let remaining_after_set = set_traffic_body
        .0
        .data
        .unwrap()
        .get("traffic_remaining_bytes")
        .unwrap()
        .as_i64()
        .unwrap();
    assert_eq!(remaining_after_set, 21474836480); // 20 GB remaining

    // --- 5. Test GET /traffic/ws-tokens ---
    let ws_tokens_resp = client
        .get(format!("{}/traffic/ws-tokens", base_url))
        .bearer_auth(&access_token1)
        .send()
        .await
        .unwrap();
    assert_eq!(ws_tokens_resp.status(), StatusCode::OK);
    let ws_tokens_body: RestApiResponse<Value> = ws_tokens_resp.json().await.unwrap();
    let ws_tokens_data = ws_tokens_body.0.data.unwrap();
    
    let connection_token = ws_tokens_data.get("connection_token").unwrap().as_str().unwrap();
    let subscription_token = ws_tokens_data.get("subscription_token").unwrap().as_str().unwrap();
    let channel = ws_tokens_data.get("channel").unwrap().as_str().unwrap();
    
    assert!(!connection_token.is_empty());
    assert!(!subscription_token.is_empty());
    assert_eq!(channel, format!("personal:{}", user_id1));
}

