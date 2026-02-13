use dhcp_template_api::{Node, Refresh, Scope, controller_service_server};
use log::info;
use tonic::{Request, Response, Status};

use crate::nodes::Nodes;

pub struct ControllerService {
    pub nodes: Nodes,
}

#[async_trait::async_trait]
impl controller_service_server::ControllerService for ControllerService {
    async fn push_node(&self, request: Request<Node>) -> Result<Response<Refresh>, Status> {
        let node = request.into_inner();

        info!("Received node update {:#?}", node);

        if self.nodes.needs_refresh(&node).await {
            if node.scope() == Scope::Shallow {
                let refresh = Refresh {
                    backoff_seconds: 0,
                    scope: Scope::Full.into(),
                };

                info!("Requesting immediate full refresh.");
                return Ok(refresh.into());
            }

            info!("Saving node to cache.");
            self.nodes.insert(node).await;
        }

        let refresh = Refresh {
            backoff_seconds: 30,
            scope: Scope::Shallow.into(),
        };

        info!("Requesting regular shallow refresh.");
        Ok(refresh.into())
    }
}
