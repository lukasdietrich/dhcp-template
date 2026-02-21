mod agent;
mod provider;

use anyhow::{Context, Result};
use envconfig::Envconfig;
use tracing::debug;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt as _, util::SubscriberInitExt as _};

use crate::agent::Agent;

#[derive(Debug, Envconfig)]
struct Config {
    #[envconfig(nested)]
    agent: agent::Config,

    #[envconfig(nested)]
    provider: provider::Config,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env()?)
        .with(fmt::layer())
        .try_init()?;

    let config = Config::init_from_env().context("Could not parse agent config.")?;
    debug!("{:#?}", config);

    let agent = Agent::from(config.agent);
    let provider = config.provider.try_into()?;

    agent.run(provider).await
}
