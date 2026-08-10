use std::sync::Arc;

use crate::features::promocode::{
    GetPromoCodeInfoQuery, UsePromoCodeCommand,
};

#[derive(Clone)]
pub struct PromoCodeState {
    pub get_info: Arc<GetPromoCodeInfoQuery>,
    pub use_promocode: Arc<UsePromoCodeCommand>,
}

impl PromoCodeState {
    pub fn new(
        get_info: Arc<GetPromoCodeInfoQuery>,
        use_promocode: Arc<UsePromoCodeCommand>,
    ) -> Self {
        Self {
            get_info,
            use_promocode,
        }
    }
}
