use rand::Rng;
use std::sync::Arc;

use crate::{
    common::http::error::AppError,
    features::promocode::{
        application::ports::PromoCodeRepository,
        domain::model::{PromoCode, PromoCodeRewardType},
    },
};

const TRIAL_TRAFFIC_BYTES: i64 = 10 * 1024 * 1024 * 1024; // 10 GB
const TRIAL_DURATION_DAYS: i32 = 7;
const CODE_CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

pub struct GetOrCreateTrialPromoCodeCommand {
    repo: Arc<dyn PromoCodeRepository>,
}

impl GetOrCreateTrialPromoCodeCommand {
    pub fn new(repo: Arc<dyn PromoCodeRepository>) -> Self {
        Self { repo }
    }

    pub fn generate_5char_code() -> String {
        let mut rng = rand::rng();
        (0..5)
            .map(|_| {
                let idx = rng.random_range(0..CODE_CHARSET.len());
                CODE_CHARSET[idx] as char
            })
            .collect()
    }

    pub async fn execute(&self, user_id: Option<&str>) -> Result<PromoCode, AppError> {
        // 1. Check if user already created an active trial promo code
        if let Some(existing) = self.repo.find_active_trial_for_creator(user_id).await? {
            return Ok(existing);
        }

        // 2. Generate unique 5-character promo code
        for _ in 0..10 {
            let code = Self::generate_5char_code();
            if self.repo.find_by_code(&code).await?.is_none() {
                return self
                    .repo
                    .create_promocode(
                        &code,
                        PromoCodeRewardType::Trial,
                        TRIAL_TRAFFIC_BYTES,
                        TRIAL_DURATION_DAYS,
                        user_id,
                        30, // Code expires in 30 days if unclaimed
                    )
                    .await;
            }
        }

        Err(AppError::InternalError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use std::sync::Mutex;
    use uuid::Uuid;

    #[derive(Default)]
    struct MockPromoCodeRepository {
        codes: Mutex<Vec<PromoCode>>,
    }

    #[async_trait]
    impl PromoCodeRepository for MockPromoCodeRepository {
        async fn create_promocode(
            &self,
            code: &str,
            reward_type: PromoCodeRewardType,
            reward_bytes: i64,
            duration_days: i32,
            created_by_user_id: Option<&str>,
            expires_in_days: i64,
        ) -> Result<PromoCode, AppError> {
            let promocode = PromoCode {
                id: Uuid::new_v4(),
                code: code.to_string(),
                reward_type,
                reward_bytes,
                duration_days,
                created_by_user_id: created_by_user_id.map(|s| Uuid::parse_str(s).unwrap()),
                used_by_user_id: None,
                expires_at: Utc::now() + chrono::Duration::days(expires_in_days),
                used_at: None,
                created_at: Utc::now(),
            };
            self.codes.lock().unwrap().push(promocode.clone());
            Ok(promocode)
        }

        async fn find_by_code(&self, code: &str) -> Result<Option<PromoCode>, AppError> {
            Ok(self
                .codes
                .lock()
                .unwrap()
                .iter()
                .find(|c| c.code.eq_ignore_ascii_case(code))
                .cloned())
        }

        async fn find_active_trial_for_creator(&self, user_id: Option<&str>) -> Result<Option<PromoCode>, AppError> {
            let uid = match user_id {
                Some(id) => Uuid::parse_str(id).unwrap(),
                None => return Ok(None),
            };
            Ok(self
                .codes
                .lock()
                .unwrap()
                .iter()
                .find(|c| c.created_by_user_id == Some(uid) && !c.is_used() && !c.is_expired())
                .cloned())
        }

        async fn count_user_redeemed_reward_type(
            &self,
            _user_id: &str,
            _reward_type: PromoCodeRewardType,
        ) -> Result<i64, AppError> {
            Ok(0)
        }

        async fn mark_as_used(&self, _code_id: &Uuid, _user_id: &str) -> Result<(), AppError> {
            Ok(())
        }
    }

    #[test]
    fn test_generate_5char_code() {
        let code = GetOrCreateTrialPromoCodeCommand::generate_5char_code();
        assert_eq!(code.len(), 5);
        assert!(code.chars().all(|c| c.is_ascii_alphanumeric() && !c.is_ascii_lowercase()));
    }

    #[tokio::test]
    async fn test_get_or_create_trial_command() {
        let repo = Arc::new(MockPromoCodeRepository::default());
        let cmd = GetOrCreateTrialPromoCodeCommand::new(repo);
        let user_id = Uuid::new_v4().to_string();

        let code1 = cmd.execute(Some(&user_id)).await.unwrap();
        assert_eq!(code1.code.len(), 5);
        assert_eq!(code1.reward_type, PromoCodeRewardType::Trial);

        // Subsequent call returns the existing active trial code
        let code2 = cmd.execute(Some(&user_id)).await.unwrap();
        assert_eq!(code1.code, code2.code);

        // Unauthenticated call generates a trial code without error
        let code_anon = cmd.execute(None).await.unwrap();
        assert_eq!(code_anon.code.len(), 5);
    }
}
