use std::collections::HashMap;

#[derive(Default)]
pub struct ReportTrafficCommand;

impl ReportTrafficCommand {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute(&self, node_id: &str, user_bytes: HashMap<String, u64>) {
        // Increment bandwidth or log stats
        for (user_uuid, bytes) in user_bytes {
            tracing::info!(
                node_id = %node_id,
                user_uuid = %user_uuid,
                bytes_transferred = bytes,
                "Node traffic report received"
            );
        }
    }
}
