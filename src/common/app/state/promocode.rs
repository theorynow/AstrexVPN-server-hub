use std::sync::Arc;

use crate::features::promocode::{
    GetOrCreateTrialPromoCodeCommand, UsePromoCodeCommand,
};

#[derive(Clone)]
pub struct PromoCodeState {
    pub get_or_create_trial: Arc<GetOrCreateTrialPromoCodeCommand>,
    pub use_promocode: Arc<UsePromoCodeCommand>,
}

impl PromoCodeState {
    pub fn new(
        get_or_create_trial: Arc<GetOrCreateTrialPromoCodeCommand>,
        use_promocode: Arc<UsePromoCodeCommand>,
    ) -> Self {
        Self {
            get_or_create_trial,
            use_promocode,
        }
    }
}
