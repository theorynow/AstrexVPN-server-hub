use serde::Serialize;
use utoipa::ToSchema;
use crate::features::promocode::domain::model::PromoCode;

#[derive(Debug, Serialize, ToSchema)]
pub struct PromoCodeResponseDto {
    pub id: String,
    pub code: String,
    pub reward_type: String,
    pub reward_bytes: i64,
    pub duration_days: i32,
    pub max_uses: i32,
    pub current_uses: i32,
    pub expires_at: String,
}

impl From<PromoCode> for PromoCodeResponseDto {
    fn from(p: PromoCode) -> Self {
        Self {
            id: p.id.to_string(),
            code: p.code,
            reward_type: p.reward_type.to_string(),
            reward_bytes: p.reward_bytes,
            duration_days: p.duration_days,
            max_uses: p.max_uses,
            current_uses: p.current_uses,
            expires_at: p.expires_at.to_rfc3339(),
        }
    }
}

pub type PromoCodeDto = PromoCodeResponseDto;

#[derive(Debug, Serialize, ToSchema)]
pub struct UsePromoCodeResponseDto {
    pub reward_type: String,
    pub reward_bytes: i64,
    pub duration_days: i32,
}
