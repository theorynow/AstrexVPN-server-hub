use std::sync::Arc;

use crate::{
    common::http::error::AppError,
    features::promocode::{
        application::ports::{PromoCodeRepository, PromoTrafficService},
        domain::model::PromoCodeRewardType,
    },
};

#[derive(Debug)]
pub struct UsePromoCodeResult {
    pub reward_type: PromoCodeRewardType,
    pub reward_bytes: i64,
    pub duration_days: i32,
}

pub struct UsePromoCodeCommand {
    repo: Arc<dyn PromoCodeRepository>,
    traffic_service: Arc<dyn PromoTrafficService>,
}

impl UsePromoCodeCommand {
    pub fn new(
        repo: Arc<dyn PromoCodeRepository>,
        traffic_service: Arc<dyn PromoTrafficService>,
    ) -> Self {
        Self {
            repo,
            traffic_service,
        }
    }

    pub async fn execute(
        &self,
        user_id: &str,
        raw_code: &str,
    ) -> Result<UsePromoCodeResult, AppError> {
        let code = raw_code.trim().to_uppercase();

        if code.len() != 5 || !code.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(AppError::ValidationError(
                "Promo code must consist of exactly 5 alphanumeric characters".into(),
            ));
        }

        let promocode = self
            .repo
            .find_by_code(&code)
            .await?
            .ok_or_else(|| AppError::NotFound("Promo code not found".into()))?;

        if promocode.is_used() {
            return Err(AppError::ValidationError(
                "Promo code has already been used".into(),
            ));
        }

        if promocode.is_expired() {
            return Err(AppError::ValidationError(
                "Promo code has expired".into(),
            ));
        }

        if promocode.reward_type == PromoCodeRewardType::Trial {
            let already_redeemed = self
                .repo
                .has_user_redeemed_reward_type(user_id, PromoCodeRewardType::Trial)
                .await?;
            if already_redeemed {
                return Err(AppError::ValidationError(
                    "You have already redeemed a trial promo code".into(),
                ));
            }
        }

        // Grant reward via PromoTrafficService cross-feature adapter
        self.traffic_service
            .grant_traffic(user_id, promocode.reward_bytes, promocode.duration_days as i64)
            .await?;

        // Mark promo code as used
        self.repo.mark_as_used(&promocode.id, user_id).await?;

        Ok(UsePromoCodeResult {
            reward_type: promocode.reward_type,
            reward_bytes: promocode.reward_bytes,
            duration_days: promocode.duration_days,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use std::sync::Mutex;
    use uuid::Uuid;
    use crate::features::promocode::domain::model::PromoCode;

    #[derive(Default)]
    struct MockPromoCodeRepository {
        codes: Mutex<Vec<PromoCode>>,
        redeemed_users: Mutex<Vec<(String, PromoCodeRewardType)>>,
    }

    #[async_trait]
    impl PromoCodeRepository for MockPromoCodeRepository {
        async fn create_promocode(
            &self,
            _code: &str,
            _reward_type: PromoCodeRewardType,
            _reward_bytes: i64,
            _duration_days: i32,
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

        async fn find_active_trial_for_creator(&self, _user_id: &str) -> Result<Option<PromoCode>, AppError> {
            unimplemented!()
        }

        async fn has_user_redeemed_reward_type(
            &self,
            user_id: &str,
            reward_type: PromoCodeRewardType,
        ) -> Result<bool, AppError> {
            Ok(self
                .redeemed_users
                .lock()
                .unwrap()
                .iter()
                .any(|(u, r)| u == user_id && *r == reward_type))
        }

        async fn mark_as_used(&self, code_id: &Uuid, user_id: &str) -> Result<(), AppError> {
            let mut list = self.codes.lock().unwrap();
            if let Some(c) = list.iter_mut().find(|c| c.id == *code_id) {
                c.used_at = Some(Utc::now());
                c.used_by_user_id = Some(Uuid::parse_str(user_id).unwrap());
            }
            self.redeemed_users
                .lock()
                .unwrap()
                .push((user_id.to_string(), PromoCodeRewardType::Trial));
            Ok(())
        }
    }

    #[derive(Default)]
    struct MockPromoTrafficService {
        granted: Mutex<Vec<(String, i64, i64)>>,
    }

    #[async_trait]
    impl PromoTrafficService for MockPromoTrafficService {
        async fn grant_traffic(&self, user_id: &str, bytes: i64, duration_days: i64) -> Result<(), AppError> {
            self.granted.lock().unwrap().push((user_id.to_string(), bytes, duration_days));
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_use_promocode_success() {
        let repo = Arc::new(MockPromoCodeRepository::default());
        let traffic_service = Arc::new(MockPromoTrafficService::default());

        let code_id = Uuid::new_v4();
        let promocode = PromoCode {
            id: code_id,
            code: "TEST1".to_string(),
            reward_type: PromoCodeRewardType::Trial,
            reward_bytes: 10 * 1024 * 1024 * 1024,
            duration_days: 7,
            created_by_user_id: None,
            used_by_user_id: None,
            expires_at: Utc::now() + chrono::Duration::days(30),
            used_at: None,
            created_at: Utc::now(),
        };
        repo.codes.lock().unwrap().push(promocode);

        let cmd = UsePromoCodeCommand::new(repo.clone(), traffic_service.clone());
        let user_id = Uuid::new_v4().to_string();

        let result = cmd.execute(&user_id, "test1").await.unwrap();
        assert_eq!(result.reward_type, PromoCodeRewardType::Trial);
        assert_eq!(result.duration_days, 7);

        // Check traffic granted
        let granted = traffic_service.granted.lock().unwrap();
        assert_eq!(granted.len(), 1);
        assert_eq!(granted[0].0, user_id);
        assert_eq!(granted[0].1, 10 * 1024 * 1024 * 1024);
        assert_eq!(granted[0].2, 7);

        // Try redeeming again should fail as used
        let err = cmd.execute(&user_id, "TEST1").await.unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)));
    }

    #[tokio::test]
    async fn test_use_promocode_invalid_length() {
        let repo = Arc::new(MockPromoCodeRepository::default());
        let traffic_service = Arc::new(MockPromoTrafficService::default());
        let cmd = UsePromoCodeCommand::new(repo, traffic_service);

        let err = cmd.execute("user1", "TOOLONG123").await.unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)));
    }
}
