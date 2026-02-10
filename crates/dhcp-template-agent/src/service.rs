use dhcp_template_api::{
    Empty, Node,
    agent_service_server::{self},
};
use tonic::{Request, Response, Status};

use crate::source::Source;

#[derive(Debug)]
pub struct AgentService {
    pub node_name: String,
    pub source: Box<dyn Source>,
}

#[tonic::async_trait]
impl agent_service_server::AgentService for AgentService {
    async fn get_node(&self, _request: Request<Empty>) -> Result<Response<Node>, Status> {
        let interfaces = self
            .source
            .get_node()
            .await
            .map_err(|err| Status::internal(err.to_string()))?;

        let node = Node {
            name: self.node_name.clone(),
            interfaces,
        };

        Ok(Response::new(node))
    }
}
