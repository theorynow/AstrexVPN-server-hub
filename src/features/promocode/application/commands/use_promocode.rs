use std::sync::Arc;

use crate::{
    common::http::error::AppError,
    features::promocode::{
        application::ports::{AbuseShieldService, PromoCodeRepository, PromoTrafficService},
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
    promo_traffic_service: Arc<dyn PromoTrafficService>,
    abuse_shield_service: Arc<dyn AbuseShieldService>,
}

impl UsePromoCodeCommand {
    pub fn new(
        repo: Arc<dyn PromoCodeRepository>,
        promo_traffic_service: Arc<dyn PromoTrafficService>,
        abuse_shield_service: Arc<dyn AbuseShieldService>,
    ) -> Self {
        Self {
            repo,
            promo_traffic_service,
            abuse_shield_service,
        }
    }

    pub async fn execute(
        &self,
        user_id: &str,
        raw_code: &str,
    ) -> Result<UsePromoCodeResult, AppError> {
        let code = raw_code.trim().to_uppercase();

        if code.len() != 5 || !code.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(AppError::PromoCodeInvalidFormat);
        }

        let promocode = self
            .repo
            .find_by_code(&code)
            .await?
            .ok_or(AppError::PromoCodeNotFound)?;

        if promocode.is_used() {
            return Err(AppError::PromoCodeAlreadyUsed);
        }

        if promocode.is_expired() {
            return Err(AppError::PromoCodeExpired);
        }

        if promocode.reward_type == PromoCodeRewardType::Trial {
            let count = self
                .repo
                .count_user_redeemed_reward_type(user_id, PromoCodeRewardType::Trial)
                .await?;
            if count >= 1 {
                return Err(AppError::PromoCodeTrialLimitReached);
            }

            if self.abuse_shield_service.is_device_trial_redeemed(user_id).await? {
                return Err(AppError::PromoCodeTrialLimitReached);
            }
        }

        // Grant reward via PromoTrafficService cross-feature adapter
        self.promo_traffic_service
            .grant_traffic(user_id, promocode.reward_bytes, promocode.duration_days as i64)
            .await?;

        // Mark promo code as used
        self.repo.mark_as_used(&promocode.id, user_id).await?;

        if promocode.reward_type == PromoCodeRewardType::Trial {
            let _ = self.abuse_shield_service.mark_device_trial_redeemed(user_id).await;
        }

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

        async fn find_active_trial_for_creator(&self, _user_id: Option<&str>) -> Result<Option<PromoCode>, AppError> {
            unimplemented!()
        }

        async fn count_user_redeemed_reward_type(
            &self,
            user_id: &str,
            reward_type: PromoCodeRewardType,
        ) -> Result<i64, AppError> {
            Ok(self
                .redeemed_users
                .lock()
                .unwrap()
                .iter()
                .filter(|(u, r)| u == user_id && *r == reward_type)
                .count() as i64)
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

    #[derive(Default)]
    struct MockAbuseShieldService {
        redeemed: Mutex<bool>,
    }

    #[async_trait]
    impl AbuseShieldService for MockAbuseShieldService {
        async fn is_device_trial_redeemed(&self, _user_id: &str) -> Result<bool, AppError> {
            Ok(*self.redeemed.lock().unwrap())
        }

        async fn mark_device_trial_redeemed(&self, _user_id: &str) -> Result<(), AppError> {
            *self.redeemed.lock().unwrap() = true;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_use_promocode_success() {
        let repo = Arc::new(MockPromoCodeRepository::default());
        let promo_traffic_service = Arc::new(MockPromoTrafficService::default());
        let abuse_shield_service = Arc::new(MockAbuseShieldService::default());

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

        let cmd = UsePromoCodeCommand::new(repo.clone(), promo_traffic_service.clone(), abuse_shield_service);
        let user_id = Uuid::new_v4().to_string();

        let result = cmd.execute(&user_id, "test1").await.unwrap();
        assert_eq!(result.reward_type, PromoCodeRewardType::Trial);
        assert_eq!(result.duration_days, 7);

        // Check traffic granted
        let granted = promo_traffic_service.granted.lock().unwrap();
        assert_eq!(granted.len(), 1);
        assert_eq!(granted[0].0, user_id);

        // Try redeeming again should fail as already used
        let err = cmd.execute(&user_id, "TEST1").await.unwrap_err();
        assert!(matches!(err, AppError::PromoCodeAlreadyUsed));
    }

    #[tokio::test]
    async fn test_use_promocode_trial_limit_one() {
        let repo = Arc::new(MockPromoCodeRepository::default());
        let promo_traffic_service = Arc::new(MockPromoTrafficService::default());
        let abuse_shield_service = Arc::new(MockAbuseShieldService::default());
        let cmd = UsePromoCodeCommand::new(repo.clone(), promo_traffic_service, abuse_shield_service);
        let user_id = Uuid::new_v4().to_string();

        // Add 2 trial promo codes
        for i in 1..=2 {
            let promocode = PromoCode {
                id: Uuid::new_v4(),
                code: format!("TRIA{}", i),
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
        }

        // Redeem 1st trial promo code -> OK
        assert!(cmd.execute(&user_id, "TRIA1").await.is_ok());

        // Redeem 2nd trial promo code -> FAILS with PromoCodeTrialLimitReached
        let err = cmd.execute(&user_id, "TRIA2").await.unwrap_err();
        assert!(matches!(err, AppError::PromoCodeTrialLimitReached));
    }

    #[tokio::test]
    async fn test_use_promocode_invalid_length() {
        let repo = Arc::new(MockPromoCodeRepository::default());
        let promo_traffic_service = Arc::new(MockPromoTrafficService::default());
        let abuse_shield_service = Arc::new(MockAbuseShieldService::default());
        let cmd = UsePromoCodeCommand::new(repo, promo_traffic_service, abuse_shield_service);

        let err = cmd.execute("user1", "TOOLONG123").await.unwrap_err();
        assert!(matches!(err, AppError::PromoCodeInvalidFormat));
    }
}
