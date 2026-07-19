use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};

use crate::features::nodes::{
    api::grpc_codegen::vpn::infrastructure::{
        hub_service_server::HubService, node_message, HubCommand, NodeMessage,
    },
    application::commands::{
        connect_node::ConnectNodeCommand, report_traffic::ReportTrafficCommand,
    },
    domain::{
        model::NodeStatus,
        ports::{node_commander::NodeCommander, node_repository::NodeRepository},
    },
};

pub struct MyCoreHubService {
    connect_node_cmd: Arc<ConnectNodeCommand>,
    report_traffic_cmd: Arc<ReportTrafficCommand>,
    node_repository: Arc<dyn NodeRepository>,
    grpc_commander: Arc<dyn NodeCommander>,
}

impl MyCoreHubService {
    pub fn new(
        connect_node_cmd: Arc<ConnectNodeCommand>,
        report_traffic_cmd: Arc<ReportTrafficCommand>,
        node_repository: Arc<dyn NodeRepository>,
        grpc_commander: Arc<dyn NodeCommander>,
    ) -> Self {
        Self {
            connect_node_cmd,
            report_traffic_cmd,
            node_repository,
            grpc_commander,
        }
    }
}

#[tonic::async_trait]
impl HubService for MyCoreHubService {
    type ConnectNodeStream = ReceiverStream<Result<HubCommand, Status>>;

    async fn connect_node(
        &self,
        request: Request<Streaming<NodeMessage>>,
    ) -> Result<Response<Self::ConnectNodeStream>, Status> {
        let mut stream = request.into_inner();

        // 1. Read first message to authenticate connection
        let first_msg = match stream.message().await {
            Ok(Some(msg)) => msg,
            Ok(None) => return Err(Status::invalid_argument("Empty stream")),
            Err(e) => return Err(Status::internal(format!("Stream read error: {}", e))),
        };

        let node_id = first_msg.node_id.clone();
        let auth_secret = first_msg.auth_secret.clone();

        if node_id.is_empty() || auth_secret.is_empty() {
            return Err(Status::unauthenticated("Node ID or auth secret is empty"));
        }

        // Enforce registration as part of the first message
        let registration = match first_msg.payload {
            Some(node_message::Payload::Registration(reg)) => reg,
            _ => {
                return Err(Status::invalid_argument(
                    "First message must contain NodeRegistration payload",
                ))
            }
        };

        // Extract inbound tags from the registration payload
        let inbound_tags: Vec<String> = registration
            .inbounds
            .iter()
            .map(|inbound| inbound.inbound_tag.clone())
            .filter(|tag| !tag.is_empty())
            .collect();

        // Validate the connection through our business logic
        self.connect_node_cmd
            .execute(&node_id, &auth_secret)
            .await
            .map_err(|e| Status::unauthenticated(e.to_string()))?;

        // 2. Create the bidirectional command transmission channel
        let (tx, rx) = mpsc::channel(100);

        // Register the sender and inbound tags inside the GrpcCommanderImpl adapter
        self.grpc_commander
            .register_node(node_id.clone(), tx, inbound_tags);

        // Mark online in DB immediately
        let repo = self.node_repository.clone();
        let _ = repo.update_status(&node_id, NodeStatus::Online).await;

        // 3. Spawn background driver to process subsequent messages and clean up upon termination
        let report_traffic_cmd = self.report_traffic_cmd.clone();
        let grpc_commander = self.grpc_commander.clone();
        let node_id_clone = node_id.clone();

        tokio::spawn(async move {
            tracing::info!(node_id = %node_id_clone, "Node gRPC stream connected successfully");

            // Loop and process the rest of the stream
            while let Ok(Some(msg)) = stream.message().await {
                if let Some(payload) = msg.payload {
                    match payload {
                        node_message::Payload::CommandResult(res) => {
                            let cid = res.command_id.clone();
                            grpc_commander.resolve_command(&cid, res);
                        }
                        node_message::Payload::TrafficReport(report) => {
                            report_traffic_cmd.execute(&node_id_clone, report).await;
                        }
                        node_message::Payload::Registration(_) => {
                            // Already registered, ignore or log
                        }
                    }
                }
            }

            // Cleanup when stream terminates/errors
            tracing::warn!(node_id = %node_id_clone, "Node gRPC stream disconnected");
            grpc_commander.deregister_node(&node_id_clone);
            let _ = repo
                .update_status(&node_id_clone, NodeStatus::Offline)
                .await;
        });

        let output_stream = ReceiverStream::new(rx);
        Ok(Response::new(output_stream))
    }
}
