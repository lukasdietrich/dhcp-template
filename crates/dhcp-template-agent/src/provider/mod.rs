mod dhcpcd;
mod notify;

use anyhow::Result;
use dhcp_template_api::Interface;
use dhcp_template_stream::boxed::BoxStream;
use envconfig::Envconfig;
use log::debug;
use strum::{Display, EnumString};

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
    Self: Sync + Send,
{
    fn interfaces<'a>(&'a self) -> BoxStream<'a, Result<Vec<Interface>>>;
}

impl From<Config> for Box<dyn Provider> {
    fn from(config: Config) -> Self {
        debug!("Creating provider {}.", config.implementation);

        Box::new(match config.implementation {
            Implementation::Dhcpcd => DhcpcdProvider::from(config.dhcpcd),
        })
    }
}
