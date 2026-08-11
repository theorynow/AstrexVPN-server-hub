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

        if self.repo.has_user_redeemed_code(user_id, &promocode.id).await? {
            return Err(AppError::PromoCodeAlreadyUsed);
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

        // Mark promo code as used (increments current_uses, records redemption)
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
        redemptions: Mutex<Vec<(Uuid, String)>>, // (promocode_id, user_id)
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

        async fn has_user_redeemed_code(&self, user_id: &str, promocode_id: &Uuid) -> Result<bool, AppError> {
            Ok(self
                .redemptions
                .lock()
                .unwrap()
                .iter()
                .any(|(pid, u)| pid == promocode_id && u == user_id))
        }

        async fn count_user_redeemed_reward_type(
            &self,
            user_id: &str,
            _reward_type: PromoCodeRewardType,
        ) -> Result<i64, AppError> {
            Ok(self
                .redemptions
                .lock()
                .unwrap()
                .iter()
                .filter(|(_, u)| u == user_id)
                .count() as i64)
        }

        async fn mark_as_used(&self, code_id: &Uuid, user_id: &str) -> Result<(), AppError> {
            let mut list = self.codes.lock().unwrap();
            if let Some(c) = list.iter_mut().find(|c| c.id == *code_id) {
                c.current_uses += 1;
            }
            self.redemptions
                .lock()
                .unwrap()
                .push((*code_id, user_id.to_string()));
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
        redeemed_users: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl AbuseShieldService for MockAbuseShieldService {
        async fn is_device_trial_redeemed(&self, user_id: &str) -> Result<bool, AppError> {
            Ok(self.redeemed_users.lock().unwrap().contains(&user_id.to_string()))
        }

        async fn mark_device_trial_redeemed(&self, user_id: &str) -> Result<(), AppError> {
            self.redeemed_users.lock().unwrap().push(user_id.to_string());
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
            max_uses: 1,
            current_uses: 0,
            created_by_user_id: None,
            expires_at: Utc::now() + chrono::Duration::days(30),
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
    async fn test_use_promocode_multi_use_success() {
        let repo = Arc::new(MockPromoCodeRepository::default());
        let promo_traffic_service = Arc::new(MockPromoTrafficService::default());
        let abuse_shield_service = Arc::new(MockAbuseShieldService::default());

        let code_id = Uuid::new_v4();
        let promocode = PromoCode {
            id: code_id,
            code: "MULTI".to_string(),
            reward_type: PromoCodeRewardType::Trial,
            reward_bytes: 10 * 1024 * 1024 * 1024,
            duration_days: 7,
            max_uses: 2,
            current_uses: 0,
            created_by_user_id: None,
            expires_at: Utc::now() + chrono::Duration::days(30),
            created_at: Utc::now(),
        };
        repo.codes.lock().unwrap().push(promocode);

        let cmd = UsePromoCodeCommand::new(repo.clone(), promo_traffic_service, abuse_shield_service);
        let user1 = Uuid::new_v4().to_string();
        let user2 = Uuid::new_v4().to_string();
        let user3 = Uuid::new_v4().to_string();

        // User 1 redeems -> OK (1/2)
        assert!(cmd.execute(&user1, "MULTI").await.is_ok());

        // User 1 tries redeeming same code again -> fails with PromoCodeAlreadyUsed
        let err_same = cmd.execute(&user1, "MULTI").await.unwrap_err();
        assert!(matches!(err_same, AppError::PromoCodeAlreadyUsed));

        // User 2 redeems -> OK (2/2)
        assert!(cmd.execute(&user2, "MULTI").await.is_ok());

        // User 3 tries redeeming -> fails as depleted (current_uses >= max_uses)
        let err_depleted = cmd.execute(&user3, "MULTI").await.unwrap_err();
        assert!(matches!(err_depleted, AppError::PromoCodeAlreadyUsed));
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
                max_uses: 1,
                current_uses: 0,
                created_by_user_id: None,
                expires_at: Utc::now() + chrono::Duration::days(30),
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

    #[tokio::test]
    async fn test_use_promocode_add_reward_type() {
        let repo = Arc::new(MockPromoCodeRepository::default());
        let promo_traffic_service = Arc::new(MockPromoTrafficService::default());
        let abuse_shield_service = Arc::new(MockAbuseShieldService::default());

        let code_id = Uuid::new_v4();
        let promocode = PromoCode {
            id: code_id,
            code: "ADD50".to_string(),
            reward_type: PromoCodeRewardType::Add,
            reward_bytes: 50 * 1024 * 1024 * 1024,
            duration_days: 30,
            max_uses: 10,
            current_uses: 0,
            created_by_user_id: None,
            expires_at: Utc::now() + chrono::Duration::days(30),
            created_at: Utc::now(),
        };
        repo.codes.lock().unwrap().push(promocode);

        let cmd = UsePromoCodeCommand::new(repo, promo_traffic_service.clone(), abuse_shield_service);
        let user1 = Uuid::new_v4().to_string();

        let result = cmd.execute(&user1, "ADD50").await.unwrap();
        assert_eq!(result.reward_type, PromoCodeRewardType::Add);
        assert_eq!(result.duration_days, 30);
        assert_eq!(result.reward_bytes, 50 * 1024 * 1024 * 1024);

        let granted = promo_traffic_service.granted.lock().unwrap();
        assert_eq!(granted.len(), 1);
        assert_eq!(granted[0].0, user1);
    }

    #[tokio::test]
    async fn test_use_promocode_not_found() {
        let repo = Arc::new(MockPromoCodeRepository::default());
        let promo_traffic_service = Arc::new(MockPromoTrafficService::default());
        let abuse_shield_service = Arc::new(MockAbuseShieldService::default());
        let cmd = UsePromoCodeCommand::new(repo, promo_traffic_service, abuse_shield_service);

        let err = cmd.execute("user1", "NOEXI").await.unwrap_err();
        assert!(matches!(err, AppError::PromoCodeNotFound));
    }

    #[tokio::test]
    async fn test_use_promocode_expired() {
        let repo = Arc::new(MockPromoCodeRepository::default());
        let promo_traffic_service = Arc::new(MockPromoTrafficService::default());
        let abuse_shield_service = Arc::new(MockAbuseShieldService::default());

        let code_id = Uuid::new_v4();
        let promocode = PromoCode {
            id: code_id,
            code: "EXPI1".to_string(),
            reward_type: PromoCodeRewardType::Trial,
            reward_bytes: 10 * 1024 * 1024 * 1024,
            duration_days: 7,
            max_uses: 10,
            current_uses: 0,
            created_by_user_id: None,
            expires_at: Utc::now() - chrono::Duration::days(1), // Already expired
            created_at: Utc::now() - chrono::Duration::days(30),
        };
        repo.codes.lock().unwrap().push(promocode);

        let cmd = UsePromoCodeCommand::new(repo, promo_traffic_service, abuse_shield_service);
        let err = cmd.execute("user1", "EXPI1").await.unwrap_err();
        assert!(matches!(err, AppError::PromoCodeExpired));
    }

    #[tokio::test]
    async fn test_use_promocode_non_alphanumeric() {
        let repo = Arc::new(MockPromoCodeRepository::default());
        let promo_traffic_service = Arc::new(MockPromoTrafficService::default());
        let abuse_shield_service = Arc::new(MockAbuseShieldService::default());
        let cmd = UsePromoCodeCommand::new(repo, promo_traffic_service, abuse_shield_service);

        let err = cmd.execute("user1", "TR!AL").await.unwrap_err();
        assert!(matches!(err, AppError::PromoCodeInvalidFormat));
    }

    #[tokio::test]
    async fn test_use_promocode_device_abuse_shield_blocked() {
        let repo = Arc::new(MockPromoCodeRepository::default());
        let promo_traffic_service = Arc::new(MockPromoTrafficService::default());
        let abuse_shield_service = Arc::new(MockAbuseShieldService::default());

        let user_id = Uuid::new_v4().to_string();
        // Simulate device ALREADY marked as trial redeemed in abuse shield
        abuse_shield_service.mark_device_trial_redeemed(&user_id).await.unwrap();

        let promocode = PromoCode {
            id: Uuid::new_v4(),
            code: "TRIA3".to_string(),
            reward_type: PromoCodeRewardType::Trial,
            reward_bytes: 10 * 1024 * 1024 * 1024,
            duration_days: 7,
            max_uses: 10,
            current_uses: 0,
            created_by_user_id: None,
            expires_at: Utc::now() + chrono::Duration::days(30),
            created_at: Utc::now(),
        };
        repo.codes.lock().unwrap().push(promocode);

        let cmd = UsePromoCodeCommand::new(repo, promo_traffic_service, abuse_shield_service);
        let err = cmd.execute(&user_id, "TRIA3").await.unwrap_err();
        assert!(matches!(err, AppError::PromoCodeTrialLimitReached));
    }

    #[tokio::test]
    async fn test_use_promocode_add_type_allows_multiple_codes() {
        let repo = Arc::new(MockPromoCodeRepository::default());
        let promo_traffic_service = Arc::new(MockPromoTrafficService::default());
        let abuse_shield_service = Arc::new(MockAbuseShieldService::default());

        // Add 2 ADD promo codes
        for i in 1..=2 {
            let promocode = PromoCode {
                id: Uuid::new_v4(),
                code: format!("ADD0{}", i),
                reward_type: PromoCodeRewardType::Add,
                reward_bytes: 5 * 1024 * 1024 * 1024,
                duration_days: 30,
                max_uses: 10,
                current_uses: 0,
                created_by_user_id: None,
                expires_at: Utc::now() + chrono::Duration::days(30),
                created_at: Utc::now(),
            };
            repo.codes.lock().unwrap().push(promocode);
        }

        let cmd = UsePromoCodeCommand::new(repo, promo_traffic_service.clone(), abuse_shield_service);
        let user_id = Uuid::new_v4().to_string();

        // User can redeem 1st ADD code
        assert!(cmd.execute(&user_id, "ADD01").await.is_ok());

        // User CAN ALSO redeem 2nd distinct ADD code (ADD does not block secondary ADD codes)
        assert!(cmd.execute(&user_id, "ADD02").await.is_ok());

        let granted = promo_traffic_service.granted.lock().unwrap();
        assert_eq!(granted.len(), 2);
    }
}
