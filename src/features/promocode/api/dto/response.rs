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
pub struct PromoCodeInfoDto {
    pub id: String,
    pub code: String,
    pub reward_type: String,
    pub reward_bytes: i64,
    pub duration_days: i32,
    pub max_uses: i32,
    pub current_uses: i32,
    pub remaining_uses: i32,
    pub is_expired: bool,
    pub is_used: bool,
    pub expires_at: String,
}

impl From<PromoCode> for PromoCodeInfoDto {
    fn from(p: PromoCode) -> Self {
        let remaining_uses = (p.max_uses - p.current_uses).max(0);
        let is_expired = p.is_expired();
        let is_used = p.is_used();
        Self {
            id: p.id.to_string(),
            code: p.code,
            reward_type: p.reward_type.to_string(),
            reward_bytes: p.reward_bytes,
            duration_days: p.duration_days,
            max_uses: p.max_uses,
            current_uses: p.current_uses,
            remaining_uses,
            is_expired,
            is_used,
            expires_at: p.expires_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UsePromoCodeResponseDto {
    pub reward_type: String,
    pub reward_bytes: i64,
    pub duration_days: i32,
}
