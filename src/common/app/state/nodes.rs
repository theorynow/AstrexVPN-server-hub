use std::sync::Arc;

use crate::features::nodes::{
    application::ports::UserTrafficService,
    domain::ports::{node_commander::NodeCommander, node_repository::NodeRepository},
};

#[derive(Clone)]
pub struct NodesState {
    pub node_repository: Arc<dyn NodeRepository>,
    pub node_commander: Arc<dyn NodeCommander>,
    pub user_traffic_service: Arc<dyn UserTrafficService>,
}

impl NodesState {
    pub fn new(
        node_repository: Arc<dyn NodeRepository>,
        node_commander: Arc<dyn NodeCommander>,
        user_traffic_service: Arc<dyn UserTrafficService>,
    ) -> Self {
        Self {
            node_repository,
            node_commander,
            user_traffic_service,
        }
    }
}
