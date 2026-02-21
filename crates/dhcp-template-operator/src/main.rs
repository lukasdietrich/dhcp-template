mod api_ext;
mod discovery;
mod operator;
mod service;
mod state;
mod template;

use std::net::SocketAddr;

use anyhow::{Context as _, Result};
use dhcp_template_api::controller_service_server::ControllerServiceServer;
use envconfig::Envconfig;
use tokio::try_join;
use tonic::transport::Server;
use tracing::{debug, info};
use tracing_subscriber::{
    EnvFilter,
    fmt::{self},
    layer::SubscriberExt as _,
    util::SubscriberInitExt as _,
};

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
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env()?)
        .with(fmt::layer())
        .try_init()?;

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
