pub mod api;
pub mod application;
pub mod domain;
pub mod infra;

pub use api::routes::{promocode_routes, PromoCodeApiDoc};
pub use application::commands::{
    get_or_create_trial::GetOrCreateTrialPromoCodeCommand,
    use_promocode::UsePromoCodeCommand,
};
pub use application::ports::{PromoCodeRepository, PromoTrafficService};
pub use domain::model::{PromoCode, PromoCodeRewardType};
pub use infra::adapters::PgPromoCodeRepository;
