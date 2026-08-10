pub mod api;
pub mod application;
pub mod domain;
pub mod infra;

pub use api::routes::{promocode_routes, PromoCodeApiDoc};
pub use application::commands::use_promocode::UsePromoCodeCommand;
pub use application::queries::get_promocode_info::GetPromoCodeInfoQuery;
pub use application::ports::{AbuseShieldService, PromoCodeRepository, PromoTrafficService};
pub use domain::model::{PromoCode, PromoCodeRewardType};
pub use infra::adapters::PgPromoCodeRepository;
