use crate::features::nodes::api::grpc_codegen::vpn::infrastructure::TrafficReport;

#[derive(Default)]
pub struct ReportTrafficCommand;

impl ReportTrafficCommand {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute(&self, node_id: &str, report: TrafficReport) {
        // Increment bandwidth or log stats
        for (user_uuid, bytes) in report.user_bytes {
            tracing::info!(
                node_id = %node_id,
                user_uuid = %user_uuid,
                bytes_transferred = bytes,
                "Node traffic report received"
            );
        }
    }
}
