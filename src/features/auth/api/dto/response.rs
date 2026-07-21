use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct RegisterResponseDto {
    pub user_id: String,
}
