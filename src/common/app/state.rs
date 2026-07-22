use axum::extract::FromRef;
use sqlx::PgPool;

use super::config::Config;

pub mod auth;
pub mod nodes;
pub mod user;
pub mod traffic;

pub use auth::AuthState;
pub use nodes::NodesState;
pub use user::UserState;
pub use traffic::TrafficState;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub pool: PgPool,
    pub auth: AuthState,
    pub user: UserState,
    pub nodes: NodesState,
    pub traffic: TrafficState,
}

impl AppState {
    pub fn new(
        config: Config,
        pool: PgPool,
        auth: AuthState,
        user: UserState,
        nodes: NodesState,
        traffic: TrafficState,
    ) -> Self {
        Self {
            config,
            pool,
            auth,
            user,
            nodes,
            traffic,
        }
    }
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

impl FromRef<AppState> for AuthState {
    fn from_ref(state: &AppState) -> Self {
        state.auth.clone()
    }
}

impl FromRef<AppState> for UserState {
    fn from_ref(state: &AppState) -> Self {
        state.user.clone()
    }
}

impl FromRef<AppState> for NodesState {
    fn from_ref(state: &AppState) -> Self {
        state.nodes.clone()
    }
}

impl FromRef<AppState> for TrafficState {
    fn from_ref(state: &AppState) -> Self {
        state.traffic.clone()
    }
}
