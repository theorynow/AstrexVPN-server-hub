use std::sync::Arc;

use crate::features::nodes::domain::ports::{
    node_commander::NodeCommander, node_repository::NodeRepository,
};

#[derive(Clone)]
pub struct NodesState {
    pub node_repository: Arc<dyn NodeRepository>,
    pub grpc_commander: Arc<dyn NodeCommander>,
}

impl NodesState {
    pub fn new(
        node_repository: Arc<dyn NodeRepository>,
        grpc_commander: Arc<dyn NodeCommander>,
    ) -> Self {
        Self {
            node_repository,
            grpc_commander,
        }
    }
}
