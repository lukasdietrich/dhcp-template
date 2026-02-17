mod operator;
mod service;
mod state;
mod template;

use std::net::SocketAddr;

use anyhow::{Context as _, Result};
use dhcp_template_api::controller_service_server::ControllerServiceServer;
use envconfig::Envconfig;
use log::{LevelFilter, debug, info};
use tokio::try_join;
use tonic::transport::Server;

use crate::{operator::Operator, service::ControllerService, state::State};

#[derive(Debug, Envconfig)]
struct Config {
    #[envconfig(from = "DHCP_TEMPLATE__ADDR", default = "[::1]:50051")]
    addr: SocketAddr,

    #[envconfig(nested)]
    state: state::Config,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Info)
        .filter_module(module_path!(), LevelFilter::Debug)
        .init();

    let config = Config::init_from_env().context("Could not parse agent config.")?;
    debug!("{:#?}", config);

    let state = State::from(config.state);

    let serve = async {
        info!("Listening on {}.", &config.addr);

        Server::builder()
            .add_service(ControllerServiceServer::new(ControllerService::new(
                state.clone(),
            )))
            .serve(config.addr)
            .await
            .context("Could not start grpc server!")
    };

    let reconcile = async {
        info!("Starting operator.");

        Operator::new(state.clone())
            .await?
            .run()
            .await
            .context("Could not start operator!")
    };

    let _ = try_join!(serve, reconcile)?;
    Ok(())
}
