use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::{
    common::http::error::AppError,
    features::nodes::domain::{
        model::{HysteriaConfig, Node, NodeStatus, XrayConfig},
        ports::node_repository::NodeRepository,
    },
};

#[derive(Clone)]
pub struct PgNodeRepository {
    pool: PgPool,
}

impl PgNodeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(Debug, FromRow)]
struct NodeRow {
    id: String,
    public_ip: String,
    name_en: String,
    country_code: String,
    country_flag: String,
    xray: Option<sqlx::types::Json<XrayConfig>>,
    hysteria: Option<sqlx::types::Json<HysteriaConfig>>,
    status: String,
    last_seen_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    modified_at: DateTime<Utc>,
}

impl From<NodeRow> for Node {
    fn from(row: NodeRow) -> Self {
        Self {
            id: row.id,
            public_ip: row.public_ip,
            name_en: row.name_en,
            country_code: row.country_code,
            country_flag: row.country_flag,
            xray: row.xray.map(|j| j.0),
            hysteria: row.hysteria.map(|j| j.0),
            status: row.status.parse().unwrap(),
            last_seen_at: row.last_seen_at,
            created_at: row.created_at,
            modified_at: row.modified_at,
        }
    }
}

#[async_trait]
impl NodeRepository for PgNodeRepository {
    async fn save(&self, node: &Node) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO nodes (id, public_ip, name_en, country_code, country_flag, xray, hysteria, status, last_seen_at, created_at, modified_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (id) DO UPDATE SET
                public_ip = EXCLUDED.public_ip,
                name_en = EXCLUDED.name_en,
                country_code = EXCLUDED.country_code,
                country_flag = EXCLUDED.country_flag,
                xray = EXCLUDED.xray,
                hysteria = EXCLUDED.hysteria,
                status = EXCLUDED.status,
                last_seen_at = EXCLUDED.last_seen_at,
                modified_at = EXCLUDED.modified_at
            "#,
        )
        .bind(&node.id)
        .bind(&node.public_ip)
        .bind(&node.name_en)
        .bind(&node.country_code)
        .bind(&node.country_flag)
        .bind(node.xray.as_ref().map(sqlx::types::Json))
        .bind(node.hysteria.as_ref().map(sqlx::types::Json))
        .bind(node.status.to_string())
        .bind(node.last_seen_at)
        .bind(node.created_at)
        .bind(node.modified_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Node>, AppError> {
        let row = sqlx::query_as::<_, NodeRow>(
            r#"
            SELECT id, public_ip, name_en, country_code, country_flag, xray, hysteria, status, last_seen_at, created_at, modified_at
            FROM nodes
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    async fn update_status(&self, id: &str, status: NodeStatus) -> Result<(), AppError> {
        let last_seen_update = if status == NodeStatus::Online {
            Some(Utc::now())
        } else {
            None
        };

        if let Some(last_seen) = last_seen_update {
            sqlx::query(
                r#"
                UPDATE nodes
                SET status = $2, last_seen_at = $3, modified_at = now()
                WHERE id = $1
                "#,
            )
            .bind(id)
            .bind(status.to_string())
            .bind(last_seen)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                r#"
                UPDATE nodes
                SET status = $2, modified_at = now()
                WHERE id = $1
                "#,
            )
            .bind(id)
            .bind(status.to_string())
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    async fn get_active_nodes(&self) -> Result<Vec<Node>, AppError> {
        let rows = sqlx::query_as::<_, NodeRow>(
            r#"
            SELECT id, public_ip, name_en, country_code, country_flag, xray, hysteria, status, last_seen_at, created_at, modified_at
            FROM nodes
            WHERE status = 'online'
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }
}
