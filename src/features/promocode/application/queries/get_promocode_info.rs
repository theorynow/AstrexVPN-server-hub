use std::sync::Arc;

use crate::{
    common::http::error::AppError,
    features::promocode::{
        application::ports::PromoCodeRepository,
        domain::model::PromoCode,
    },
};

pub struct GetPromoCodeInfoQuery {
    repo: Arc<dyn PromoCodeRepository>,
}

impl GetPromoCodeInfoQuery {
    pub fn new(repo: Arc<dyn PromoCodeRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, raw_code: &str) -> Result<PromoCode, AppError> {
        let code = raw_code.trim().to_uppercase();

        if code.is_empty() || code.len() > 16 {
            return Err(AppError::PromoCodeInvalidFormat);
        }

        let promocode = self
            .repo
            .find_by_code(&code)
            .await?
            .ok_or(AppError::PromoCodeNotFound)?;

        Ok(promocode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use std::sync::Mutex;
    use uuid::Uuid;
    use crate::features::promocode::domain::model::PromoCodeRewardType;

    #[derive(Default)]
    struct MockPromoCodeRepository {
        codes: Mutex<Vec<PromoCode>>,
    }

    #[async_trait]
    impl PromoCodeRepository for MockPromoCodeRepository {
        async fn create_promocode(
            &self,
            _code: &str,
            _reward_type: PromoCodeRewardType,
            _reward_bytes: i64,
            _duration_days: i32,
            _max_uses: i32,
            _created_by_user_id: Option<&str>,
            _expires_in_days: i64,
        ) -> Result<PromoCode, AppError> {
            unimplemented!()
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

        async fn find_active_trial_for_creator(&self, _user_id: Option<&str>) -> Result<Option<PromoCode>, AppError> {
            unimplemented!()
        }

        async fn has_user_redeemed_code(&self, _user_id: &str, _promocode_id: &Uuid) -> Result<bool, AppError> {
            Ok(false)
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

    #[tokio::test]
    async fn test_get_promocode_info_success() {
        let repo = Arc::new(MockPromoCodeRepository::default());
        let code_id = Uuid::new_v4();
        let promocode = PromoCode {
            id: code_id,
            code: "INFO1".to_string(),
            reward_type: PromoCodeRewardType::Trial,
            reward_bytes: 10 * 1024 * 1024 * 1024,
            duration_days: 7,
            max_uses: 10,
            current_uses: 3,
            created_by_user_id: None,
            expires_at: Utc::now() + chrono::Duration::days(30),
            created_at: Utc::now(),
        };
        repo.codes.lock().unwrap().push(promocode);

        let query = GetPromoCodeInfoQuery::new(repo);
        let res = query.execute("info1").await.unwrap();
        assert_eq!(res.code, "INFO1");
        assert_eq!(res.max_uses, 10);
        assert_eq!(res.current_uses, 3);
    }

    #[tokio::test]
    async fn test_get_promocode_info_not_found() {
        let repo = Arc::new(MockPromoCodeRepository::default());
        let query = GetPromoCodeInfoQuery::new(repo);
        let err = query.execute("NONEXISTENT").await.unwrap_err();
        assert!(matches!(err, AppError::PromoCodeNotFound));
    }
}
