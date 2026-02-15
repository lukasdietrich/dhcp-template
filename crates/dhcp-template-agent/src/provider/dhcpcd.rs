use std::{
    collections::BTreeMap,
    fs::{self, canonicalize, read},
    path::{Path, PathBuf},
};

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use dhcp_template_api::{Interface, Lease4, Lease6};
use envconfig::Envconfig;
use log::info;

use crate::provider::notify::{InterfaceReader, NotifyProvider};

#[derive(Debug, Envconfig)]
pub struct Config {
    #[envconfig(from = "DHCP_TEMPLATE__DHCPCD_PATH", default = "/var/lib/dhcpcd")]
    path: PathBuf,
}

pub type DhcpcdProvider = NotifyProvider<DhcpcdInterfaceReader>;

impl TryFrom<Config> for DhcpcdProvider {
    type Error = anyhow::Error;

    fn try_from(config: Config) -> Result<Self, Self::Error> {
        let path = canonicalize(config.path)?;
        let provider = Self::new(path, DhcpcdInterfaceReader);

        Ok(provider)
    }
}

pub struct DhcpcdInterfaceReader;

#[async_trait]
impl InterfaceReader for DhcpcdInterfaceReader {
    async fn interfaces(&self, path: &Path) -> Result<Vec<Interface>> {
        info!("Reading lease files in {:?}.", path);

        let lease_files = fs::read_dir(path)?
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| v4::is_lease_file(path) || v6::is_lease_file(path));

        let mut interfaces = BTreeMap::new();

        for lease_file in lease_files {
            info!("Reading lease file {:?}.", lease_file);

            let name = file_base_name(&lease_file)
                .ok_or_else(|| anyhow!("Could not extract base name from path {:?}.", &path))?;

            let interface = interfaces.entry(name.clone()).or_insert(Interface {
                name,
                lease4: Default::default(),
                lease6: Default::default(),
            });

            match parse_lease(&lease_file)? {
                Lease::V4(lease4) => interface.lease4 = Some(lease4),
                Lease::V6(lease6) => interface.lease6 = Some(lease6),
            };
        }

        Ok(interfaces.values().cloned().collect())
    }
}

fn file_base_name(path: &Path) -> Option<String> {
    path.file_prefix()
        .and_then(|s| s.to_str())
        .map(|s| s.to_owned())
}

enum Lease {
    V4(Lease4),
    V6(Lease6),
}

fn parse_lease(path: &Path) -> Result<Lease> {
    let bytes = read(path)?;

    let lease = if v4::is_lease_file(path) {
        Lease::V4(v4::decode(&bytes)?)
    } else {
        Lease::V6(v6::decode(&bytes)?)
    };

    Ok(lease)
}

mod v4 {
    use std::path::Path;

    use anyhow::{Context as _, Result};
    use dhcp_template_api::Lease4;
    use dhcproto::{
        Decodable as _, Decoder,
        v4::{DhcpOption, DhcpOptions, Message, OptionCode},
    };

    pub fn decode(bytes: &[u8]) -> Result<Lease4> {
        let mut decoder = Decoder::new(bytes);
        let message = Message::decode(&mut decoder).context("Could not decode dhcpv4 message.")?;
        let options = message.opts();

        let lease = Lease4 {
            dns: get_dns(options),
            domain: get_domain(options),
        };

        Ok(lease)
    }

    fn get_dns(options: &DhcpOptions) -> Vec<String> {
        if let Some(DhcpOption::DomainNameServer(dns)) = options.get(OptionCode::DomainNameServer) {
            dns.iter().map(|addr| addr.to_string()).collect()
        } else {
            Default::default()
        }
    }

    fn get_domain(options: &DhcpOptions) -> Option<String> {
        if let Some(DhcpOption::DomainName(domain)) = options.get(OptionCode::DomainName) {
            Some(domain.clone())
        } else {
            None
        }
    }

    pub fn is_lease_file(path: &Path) -> bool {
        path.extension()
            .is_some_and(|ext| matches!(ext.to_str(), Some("lease")))
    }
}

mod v6 {
    use std::path::Path;

    use anyhow::{Context as _, Result};
    use dhcp_template_api::{Lease6, Prefix6};
    use dhcproto::{
        Decodable as _, Decoder,
        v6::{DhcpOption, DhcpOptions, Message, OptionCode},
    };

    pub fn decode(bytes: &[u8]) -> Result<Lease6> {
        let mut decoder = Decoder::new(bytes);
        let message = Message::decode(&mut decoder).context("Could not decode dhcpv6 message.")?;
        let options = message.opts();

        let lease = Lease6 {
            dns: get_dns(options),
            prefix6: get_prefix(options),
        };

        Ok(lease)
    }

    fn get_dns(options: &DhcpOptions) -> Vec<String> {
        options
            .get_all(OptionCode::DomainNameServers)
            .into_iter()
            .flatten()
            .filter_map(|option| {
                if let DhcpOption::DomainNameServers(dns) = option {
                    Some(dns)
                } else {
                    None
                }
            })
            .flat_map(|dns| dns.iter().map(|addr| addr.to_string()))
            .collect()
    }

    fn get_prefix(options: &DhcpOptions) -> Vec<Prefix6> {
        options
            .get_all(OptionCode::IAPD)
            .into_iter()
            .flatten()
            .filter_map(|option| {
                if let DhcpOption::IAPD(iapd) = option {
                    Some(iapd)
                } else {
                    None
                }
            })
            .filter_map(|iapd| iapd.opts.get_all(OptionCode::IAPrefix))
            .flatten()
            .filter_map(|option| {
                if let DhcpOption::IAPrefix(iaprefix) = option {
                    Some(iaprefix)
                } else {
                    None
                }
            })
            .map(|iaprefix| Prefix6 {
                ip: iaprefix.prefix_ip.to_string(),
                len: iaprefix.prefix_len as u32,
            })
            .collect()
    }

    pub fn is_lease_file(path: &Path) -> bool {
        path.extension()
            .is_some_and(|ext| matches!(ext.to_str(), Some("lease6")))
    }
}
