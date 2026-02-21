mod dhcpcd;
mod notify;

use anyhow::Result;
use dhcp_template_api::Interface;
use envconfig::Envconfig;
use futures_util::stream::BoxStream;
use strum::{Display, EnumString};
use tracing::{Level, instrument};

use crate::provider::dhcpcd::DhcpcdProvider;

#[derive(Debug, Display, EnumString)]
#[strum(ascii_case_insensitive, serialize_all = "snake_case")]
enum Implementation {
    Dhcpcd,
}

#[derive(Debug, Envconfig)]
pub struct Config {
    #[envconfig(from = "DHCP_TEMPLATE__PROVIDER", default = "dhcpcd")]
    implementation: Implementation,

    #[envconfig(nested)]
    dhcpcd: dhcpcd::Config,
}

pub trait Provider
where
    Self: std::fmt::Debug + Sync + Send,
{
    fn interfaces<'a>(&'a self) -> BoxStream<'a, Result<Vec<Interface>>>;
}

impl TryFrom<Config> for Box<dyn Provider> {
    type Error = anyhow::Error;

    #[instrument(ret(level = Level::DEBUG), err(level = Level::ERROR))]
    fn try_from(config: Config) -> Result<Self, Self::Error> {
        let provider = Box::new(match config.implementation {
            Implementation::Dhcpcd => DhcpcdProvider::try_from(config.dhcpcd)?,
        });

        Ok(provider)
    }
}
