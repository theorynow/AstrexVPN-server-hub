use common::app::{
    bootstrap::{build_app_state, run_database_migrations, shutdown_signal},
    config::{setup_database, Config},
};
use hub::{app::create_router, common};
use std::time::Instant;
use tracing::{error, info};

#[cfg(not(feature = "opentelemetry"))]
use common::app::bootstrap::setup_tracing;

#[cfg(feature = "opentelemetry")]
use common::observability::opentelemetry::{setup_tracing_opentelemetry, shutdown_opentelemetry};

/// Main entry point for the application.
/// It sets up the database connection, initializes the server, and starts listening for requests.
/// It also sets up the Swagger UI for API documentation.
///
/// # Errors
/// Returns an error if the database connection fails or if the server fails to start.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let startup_started_at = Instant::now();

    #[cfg(not(feature = "opentelemetry"))]
    setup_tracing();

    #[cfg(feature = "opentelemetry")]
    let otel_providers = {
        let providers = setup_tracing_opentelemetry();
        // Startup span to ensure at least one span is generated and exported
        let span = tracing::info_span!("startup");
        let _enter = span.enter();
        providers
    };

    info!("Loading application configuration");
    let config = match Config::from_env() {
        Ok(config) => config,
        Err(err) => {
            error!(error = %err, "Application configuration failed");
            return Err(err.into());
        }
    };

    let pool = match setup_database(&config).await {
        Ok(pool) => pool,
        Err(err) => {
            error!(error = %err, "Application startup stopped because database is unavailable");
            return Err(err.into());
        }
    };

    if let Err(err) = run_database_migrations(&pool).await {
        error!(error = %err, "Application startup stopped because database migrations failed");
        return Err(err.into());
    }

    let state = build_app_state(pool.clone(), config.clone());

    // Spawn gRPC server concurrently
    let grpc_addr: std::net::SocketAddr =
        format!("{}:{}", config.service_host, config.grpc_port).parse()?;

    let connect_node_cmd = std::sync::Arc::new(
        hub::features::nodes::application::commands::connect_node::ConnectNodeCommand::new(
            state.nodes.node_repository.clone(),
            config.node_auth_secret.clone(),
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
        info!("Starting gRPC server listening on {}", grpc_addr);
        if let Err(e) = tonic::transport::Server::builder()
            .add_service(
                hub::features::nodes::api::grpc_codegen::vpn::infrastructure::hub_service_server::HubServiceServer::new(
                    grpc_service,
                ),
            )
            .serve(grpc_addr)
            .await
        {
            error!(error = %e, "gRPC server stopped with error");
        }
    });

    let app = create_router(state);

    let addr = format!("{}:{}", config.service_host, config.service_port);

    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(listener) => {
            info!(
                addr = %addr,
                local_addr = %listener.local_addr()?,
                startup_elapsed_ms = startup_started_at.elapsed().as_millis(),
            );
            listener
        }
        Err(err) => {
            error!(addr = %addr, error = %err, "Failed to bind HTTP listener");
            return Err(err.into());
        }
    };

    let shutdown_signal_fut = async move {
        shutdown_signal().await;
        info!("Graceful shutdown initiated");
    };

    if let Err(err) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal_fut)
        .await
    {
        error!(error = %err, "HTTP server stopped with error");
        return Err(err.into());
    }

    info!("HTTP server stopped");

    #[cfg(feature = "opentelemetry")]
    if let Err(err) = shutdown_opentelemetry(otel_providers) {
        error!(error = %err, "OpenTelemetry shutdown failed");
        return Err(err);
    }

    info!(
        uptime_ms = startup_started_at.elapsed().as_millis(),
        "Application shutdown completed"
    );

    Ok(())
}
