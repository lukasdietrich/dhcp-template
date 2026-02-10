use std::net::SocketAddr;

use anyhow::Result;
use dhcp_template_api::agent_service_server::AgentServiceServer;
use envconfig::Envconfig;
use log::{LevelFilter, info};
use tonic::transport::Server;

use crate::{service::AgentService, source::dhcpcd::DhcpcdSource};

mod service;
mod source;

#[derive(Envconfig)]
struct Config {
    #[envconfig(from = "DHCP_TEMPLATE__NODE_NAME", default = "localhost")]
    node_name: String,

    #[envconfig(from = "DHCP_TEMPLATE__ADDR", default = "[::1]:50051")]
    addr: SocketAddr,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .init();

    let Config { node_name, addr } = Config::init_from_env()?;

    let agent_service = AgentService {
        node_name: node_name.clone(),
        source: Box::new(DhcpcdSource::init_from_env()?),
    };

    info!("Listening on {:?} for node {:?}", &addr, &node_name);

    Server::builder()
        .add_service(AgentServiceServer::new(agent_service))
        .serve(addr)
        .await?;

    Ok(())
}
