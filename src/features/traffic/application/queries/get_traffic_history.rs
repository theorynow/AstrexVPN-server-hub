use std::sync::Arc;

use crate::{
    common::http::error::AppError,
    features::traffic::{application::ports::TrafficRepository, domain::model::TrafficPacket},
};

pub struct GetTrafficHistoryQuery {
    traffic_repo: Arc<dyn TrafficRepository>,
}

impl GetTrafficHistoryQuery {
    pub fn new(traffic_repo: Arc<dyn TrafficRepository>) -> Self {
        Self { traffic_repo }
    }

    pub async fn execute(&self, user_id: &str) -> Result<Vec<TrafficPacket>, AppError> {
        self.traffic_repo.get_history(user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use uuid::Uuid;
    use crate::features::traffic::domain::model::TrafficSummary;

    struct MockTrafficRepository {
        packets: Vec<TrafficPacket>,
    }

    #[async_trait]
    impl TrafficRepository for MockTrafficRepository {
        async fn get_summary(&self, _user_id: &str) -> Result<TrafficSummary, AppError> {
            unimplemented!()
        }

        async fn add_packet(&self, _user_id: &str, _bytes: i64) -> Result<TrafficPacket, AppError> {
            unimplemented!()
        }

        async fn consume(&self, _user_id: &str, _bytes: u64) -> Result<u64, AppError> {
            unimplemented!()
        }

        async fn get_remaining(&self, _user_id: &str) -> Result<u64, AppError> {
            unimplemented!()
        }

        async fn get_history(&self, user_id: &str) -> Result<Vec<TrafficPacket>, AppError> {
            let uid = Uuid::parse_str(user_id).map_err(|e| AppError::ValidationError(e.to_string()))?;
            Ok(self
                .packets
                .iter()
                .filter(|p| p.user_id == uid)
                .cloned()
                .collect())
        }
    }

    #[tokio::test]
    async fn test_get_traffic_history_query() {
        let user_id = Uuid::new_v4();
        let packet1 = TrafficPacket {
            id: Uuid::new_v4(),
            user_id,
            traffic_limit_bytes: 10 * 1024 * 1024 * 1024,
            traffic_remaining_bytes: 5 * 1024 * 1024 * 1024,
            expires_at: Utc::now(),
            created_at: Utc::now(),
            modified_at: Utc::now(),
        };

        let repo = Arc::new(MockTrafficRepository {
            packets: vec![packet1.clone()],
        });
        let query = GetTrafficHistoryQuery::new(repo);

        let result = query.execute(&user_id.to_string()).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, packet1.id);
        assert_eq!(result[0].traffic_limit_bytes, packet1.traffic_limit_bytes);
    }
}
