mod nodes;
mod service;

use std::net::SocketAddr;

use dhcp_template_api::controller_service_server::ControllerServiceServer;
use envconfig::Envconfig;
use log::{LevelFilter, info};
use tonic::transport::Server;

use crate::{nodes::Nodes, service::ControllerService};

#[derive(Envconfig)]
struct Config {
    #[envconfig(from = "DHCP_TEMPLATE__ADDR", default = "[::1]:50051")]
    addr: SocketAddr,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Info)
        .filter_module("dhcp_template_operator", LevelFilter::Debug)
        .init();

    let config = Config::init_from_env()?;
    let nodes = Nodes::init();

    let controller_service = ControllerService { nodes };

    info!("Listening on {}", &config.addr);

    Server::builder()
        .add_service(ControllerServiceServer::new(controller_service))
        .serve(config.addr)
        .await?;

    Ok(())
}
