mod controller;
mod k8s;
mod service;
mod state;
mod template;

use std::net::SocketAddr;

use anyhow::{Context as _, Result};
use dhcp_template_api::controller_service_server::ControllerServiceServer;
use envconfig::Envconfig;
use tokio::try_join;
use tonic::transport::Server;
use tracing::{debug, info, level_filters::LevelFilter};
use tracing_subscriber::{
    EnvFilter,
    fmt::{self},
    layer::SubscriberExt as _,
    util::SubscriberInitExt as _,
};

use crate::{service::ControllerService, state::State};

#[derive(Debug, Envconfig)]
struct Config {
    #[envconfig(from = "DHCP_TEMPLATE__ADDR", default = "[::1]:50051")]
    addr: SocketAddr,

    #[envconfig(nested)]
    state: state::Config,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env()?,
        )
        .with(fmt::layer())
        .try_init()?;

    let config = Config::init_from_env().context("Could not parse agent config.")?;
    debug!("{:#?}", config);

    let state = State::from(config.state);

    let serve = async {
        info!("Listening on {}.", &config.addr);

        Server::builder()
            .add_service(ControllerServiceServer::new(ControllerService::from(
                state.clone(),
            )))
            .serve(config.addr)
            .await
            .context("Could not start grpc server.")
    };

    let reconcile = async {
        info!("Starting operator.");

        controller::run(state.clone())
            .await
            .context("Could not start controller.")
    };

    let _ = try_join!(serve, reconcile)?;
    Ok(())
}
