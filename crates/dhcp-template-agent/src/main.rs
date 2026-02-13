mod agent;
mod provider;

use anyhow::{Context, Result};
use envconfig::Envconfig;
use log::LevelFilter;

use crate::agent::Agent;

#[derive(Envconfig)]
struct Config {
    #[envconfig(nested)]
    agent: agent::Config,

    #[envconfig(nested)]
    provider: provider::Config,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Info)
        .filter_module(module_path!(), LevelFilter::Debug)
        .init();

    let config = Config::init_from_env().context("Could not parse agent config.")?;
    let agent = Agent::from(config.agent);
    let provider = config.provider.into();

    agent.push_node(provider).await
}
