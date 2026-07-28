use std::sync::Arc;
use std::time::Instant;

use sqlx::{migrate::MigrateError, PgPool};

use crate::common::app::{
    config::Config,
    state::{AppState, AuthState, NodesState, UserState, TrafficState},
};
use crate::features::auth::{
    AuthAsGuestCommand, AuthRepository, AuthRepositoryImpl, ChangePasswordCommand,
    LoginUserCommand, RefreshSessionCommand, RegisterUserCommand, UserExistsQuery,
};
use crate::features::nodes::{
    application::ports::UserTrafficService,
    domain::ports::node_repository::NodeRepository,
    infra::adapters::{pg_node_repository::PgNodeRepository, ws_commander_impl::WsCommanderImpl},
};
use crate::features::user::{
    GetMeQuery, GetUserByIdQuery, GetUserListQuery, GetUsersQuery, UpdateMeCommand,
    UserAuthService, UserRepository, UserRepositoryImpl,
};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Runs database migrations before the application starts handling requests.
pub async fn run_database_migrations(pool: &PgPool) -> Result<(), MigrateError> {
    let started_at = Instant::now();

    tracing::info!("Database migrations started");

    match sqlx::migrate!("./migrations").run(pool).await {
        Ok(()) => {
            tracing::info!(
                elapsed_ms = started_at.elapsed().as_millis(),
                "Database migrations completed"
            );
            Ok(())
        }
        Err(err) => {
            tracing::error!(
                elapsed_ms = started_at.elapsed().as_millis(),
                error = %err,
                "Database migrations failed"
            );
            Err(err)
        }
    }
}

/// Constructs and wires all application services and returns a configured AppState.
pub fn build_app_state(pool: PgPool, config: Config) -> AppState {
    tracing::info!("Building application state");

    // Auth
    let auth_repository: Arc<dyn AuthRepository> = Arc::new(AuthRepositoryImpl::new(pool.clone()));
    let register_user = Arc::new(RegisterUserCommand::new(auth_repository.clone()));
    let login_user = Arc::new(LoginUserCommand::new(auth_repository.clone()));
    let auth_as_guest = Arc::new(AuthAsGuestCommand::new(auth_repository.clone()));
    let user_exists = Arc::new(UserExistsQuery::new(auth_repository.clone()));
    let change_password = Arc::new(ChangePasswordCommand::new(auth_repository.clone()));
    let refresh_session = Arc::new(RefreshSessionCommand::new(auth_repository.clone()));

    // Traffic
    let realtime_publisher = Arc::new(crate::common::app::adapters::HttpCentrifugoClient::new(config.clone()));
    let pg_traffic_repo = Arc::new(crate::features::traffic::PgTrafficRepository::new(pool.clone(), realtime_publisher));
    let traffic_repository: Arc<dyn crate::features::traffic::TrafficRepository> = pg_traffic_repo.clone();

    let add_traffic = Arc::new(crate::features::traffic::AddTrafficCommand::new(traffic_repository.clone()));
    let subtract_traffic = Arc::new(crate::features::traffic::SubtractTrafficCommand::new(traffic_repository.clone()));
    let set_traffic = Arc::new(crate::features::traffic::SetTrafficCommand::new(traffic_repository.clone()));
    let consume_traffic = Arc::new(crate::features::traffic::ConsumeTrafficCommand::new(traffic_repository.clone()));
    let get_ws_tokens = Arc::new(crate::features::traffic::GetWsTokensCommand::new());
    let get_summary = Arc::new(crate::features::traffic::GetTrafficSummaryQuery::new(traffic_repository.clone()));
    let get_remaining_traffic = Arc::new(crate::features::traffic::GetRemainingTrafficQuery::new(traffic_repository.clone()));

    let traffic_state = TrafficState::new(
        add_traffic,
        subtract_traffic,
        set_traffic,
        get_ws_tokens,
        get_summary,
    );

    // Cross-feature adapter for Nodes -> Traffic
    let user_traffic_service: Arc<dyn UserTrafficService> = Arc::new(
        crate::common::app::adapters::UserTrafficServiceAdapter::new(
            consume_traffic,
            get_remaining_traffic,
        )
    );

    // Cross-feature adapter for User -> Auth
    let user_auth_service: Arc<dyn UserAuthService> = Arc::new(
        crate::common::app::adapters::UserAuthServiceAdapter::new(
            change_password.clone(),
            auth_repository.clone(),
        )
    );

    // User
    let user_repository: Arc<dyn UserRepository> = Arc::new(UserRepositoryImpl::new(pool.clone()));
    let update_me = Arc::new(UpdateMeCommand::new(user_repository.clone(), user_auth_service.clone()));
    let get_me = Arc::new(GetMeQuery::new(user_repository.clone(), user_auth_service.clone()));
    let get_user_by_id = Arc::new(GetUserByIdQuery::new(user_repository.clone(), user_auth_service.clone()));
    let get_user_list = Arc::new(GetUserListQuery::new(user_repository.clone(), user_auth_service.clone()));
    let get_users = Arc::new(GetUsersQuery::new(user_repository.clone(), user_auth_service));

    let max_file_size_bytes = config.max_file_size_mb as usize * 1024 * 1024;

    // States
    let auth_state = AuthState::new(
        register_user,
        login_user,
        auth_as_guest,
        refresh_session,
        user_exists,
        change_password,
    );
    let user_state = UserState::new(
        update_me,
        get_me,
        get_user_by_id,
        get_user_list,
        get_users,
        max_file_size_bytes,
    );

    let nodes_repo: Arc<dyn NodeRepository> = Arc::new(PgNodeRepository::new(pool.clone()));
    let node_commander = Arc::new(WsCommanderImpl::new());
    let nodes_state = NodesState::new(nodes_repo, node_commander, user_traffic_service);

    let state = AppState::new(config, pool.clone(), auth_state, user_state, nodes_state, traffic_state);

    tracing::info!("Application state built");

    state
}

/// Setup tracing for the application.
pub fn setup_tracing() {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,sqlx=info,tower_http=info,axum::rejection=trace".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_file(true)
                .with_line_number(true)
                .with_timer(tracing_subscriber::fmt::time::ChronoLocal::rfc_3339())
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_target(true)
                .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE),
        )
        .init();
}

/// Shutdown signal handler
pub async fn shutdown_signal() {
    match tokio::signal::ctrl_c().await {
        Ok(()) => tracing::info!("Shutdown signal received"),
        Err(err) => tracing::error!(
            error = %err,
            "Failed to install CTRL+C signal handler"
        ),
    }
}
