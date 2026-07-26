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
async fn test_auth_routes_lifecycle() {
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
    let state = build_app_state(pool.clone(), config.clone());
    let app = create_router(state);
    tokio::spawn(async move {
        axum::serve(app_listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let base_url = format!("http://{}", app_addr);

    // --- 1. Test /auth/guest (gets auto-generated guest-{uuid} username) ---
    let resp = client
        .post(format!("{}/auth/guest", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let auth_body: RestApiResponse<Value> = resp.json().await.unwrap();
    let data = auth_body.0.data.unwrap();
    let access_token = data
        .get("access_token")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();
    let guest_user_id = data
        .get("user_id")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();
    assert!(!guest_user_id.is_empty());
    let refresh_token = data
        .get("refresh_token")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();

    // Verify the guest user has a guest-{uuid} username in both API response and DB
    let me_resp = client
        .get(format!("{}/user/me", base_url))
        .bearer_auth(&access_token)
        .send()
        .await
        .unwrap();
    assert_eq!(me_resp.status(), StatusCode::OK);
    let me_body: RestApiResponse<Value> = me_resp.json().await.unwrap();
    let me_data = me_body.0.data.unwrap();
    let user_id = me_data.get("id").unwrap().as_str().unwrap().to_string();
    let username = me_data
        .get("username")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();
    assert!(username.starts_with("guest-"));

    // Check DB directly — guest users now always have username = "guest-{uuid}"
    let db_username: Option<String> =
        sqlx::query_scalar("SELECT username FROM users WHERE id = $1::uuid")
            .bind(&user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        db_username
            .as_deref()
            .map(|u| u.starts_with("guest-"))
            .unwrap_or(false),
        "Guest user should have a guest-{{uuid}} username in DB, got: {:?}",
        db_username
    );

    // --- 3. Test /auth/register (register normal user) ---
    let reg_username = format!("user-{}", Uuid::new_v4());
    let reg_password = "testpassword123";
    let reg_resp = client
        .post(format!("{}/auth/register", base_url))
        .json(&json!({
            "username": reg_username,
            "password": reg_password
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(reg_resp.status(), StatusCode::OK);
    let reg_resp_body: RestApiResponse<Value> = reg_resp.json().await.unwrap();
    let reg_user_id = reg_resp_body
        .0
        .data
        .unwrap()
        .get("user_id")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();
    assert!(!reg_user_id.is_empty());

    // Try registering the same username again (should fail)
    let dup_resp = client
        .post(format!("{}/auth/register", base_url))
        .json(&json!({
            "username": reg_username,
            "password": reg_password
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(dup_resp.status(), StatusCode::CONFLICT);

    // --- 4. Test /auth/login ---
    let login_resp = client
        .post(format!("{}/auth/login", base_url))
        .json(&json!({
            "username": reg_username,
            "password": reg_password
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(login_resp.status(), StatusCode::OK);
    let login_body: RestApiResponse<Value> = login_resp.json().await.unwrap();
    let login_data = login_body.0.data.unwrap();
    let login_user_id = login_data
        .get("user_id")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(login_user_id, reg_user_id);

    let reg_access_token = login_data
        .get("access_token")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();

    // Try login with wrong password
    let bad_login_resp = client
        .post(format!("{}/auth/login", base_url))
        .json(&json!({
            "username": reg_username,
            "password": "wrongpassword"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(bad_login_resp.status(), StatusCode::UNAUTHORIZED);

    // --- 5. Test /auth/refresh ---
    let refresh_resp = client
        .post(format!("{}/auth/refresh", base_url))
        .json(&json!({
            "refresh_token": refresh_token
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(refresh_resp.status(), StatusCode::OK);
    let refresh_body: RestApiResponse<Value> = refresh_resp.json().await.unwrap();
    let new_access_token = refresh_body
        .0
        .data
        .unwrap()
        .get("access_token")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();
    assert!(!new_access_token.is_empty());

    // --- 6. Test /auth/change-password ---
    let new_password = "newsecretpassword";
    let change_pwd_resp = client
        .post(format!("{}/auth/change-password", base_url))
        .bearer_auth(&reg_access_token)
        .json(&json!({
            "new_password": new_password
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(change_pwd_resp.status(), StatusCode::OK);

    // Login with old password should now fail
    let old_pwd_login_resp = client
        .post(format!("{}/auth/login", base_url))
        .json(&json!({
            "username": reg_username,
            "password": reg_password
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(old_pwd_login_resp.status(), StatusCode::UNAUTHORIZED);

    // Login with new password should succeed
    let new_pwd_login_resp = client
        .post(format!("{}/auth/login", base_url))
        .json(&json!({
            "username": reg_username,
            "password": new_password
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(new_pwd_login_resp.status(), StatusCode::OK);

    // --- 7. Test UserTrafficService packet consumption ---
    use hub::features::nodes::application::ports::UserTrafficService;
    use std::sync::Arc;
    let publisher = Arc::new(hub::common::app::adapters::HttpCentrifugoClient::new(config.clone()));
    let pg_traffic_repo = Arc::new(hub::features::traffic::PgTrafficRepository::new(pool.clone(), publisher));
    let consume_cmd = Arc::new(hub::features::traffic::ConsumeTrafficCommand::new(pg_traffic_repo.clone()));
    let remaining_query = Arc::new(hub::features::traffic::GetRemainingTrafficQuery::new(pg_traffic_repo.clone()));
    let user_traffic_service: Arc<dyn UserTrafficService> = Arc::new(hub::common::app::adapters::UserTrafficServiceAdapter::new(consume_cmd, remaining_query));
    
    // Get remaining traffic for guest user created earlier (user_id)
    let initial_remaining = user_traffic_service.get_remaining_traffic(&user_id).await.unwrap();
    assert_eq!(initial_remaining, 26843545600); // 25 GB default packet

    // Consume 5 GB
    let consumed_bytes = 5000000000;
    let remaining_after_first = user_traffic_service.consume_traffic(&user_id, consumed_bytes).await.unwrap();
    assert_eq!(remaining_after_first, 26843545600 - 5000000000);

    // Consume all remaining and more (30 GB)
    let remaining_after_second = user_traffic_service.consume_traffic(&user_id, 30000000000).await.unwrap();
    assert_eq!(remaining_after_second, 0);

    // Try to consume when already at 0
    let remaining_after_third = user_traffic_service.consume_traffic(&user_id, 1000).await.unwrap();
    assert_eq!(remaining_after_third, 0);
}
