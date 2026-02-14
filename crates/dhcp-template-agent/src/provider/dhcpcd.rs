use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use dhcp_template_api::{Interface, Lease4, Lease6, Prefix6};
use dhcproto::{Decodable, v4, v6};
use envconfig::Envconfig;
use log::info;

use crate::provider::notify::{InterfaceReader, NotifyProvider};

#[derive(Debug, Envconfig)]
pub struct Config {
    #[envconfig(from = "DHCP_TEMPLATE__DHCPCD_PATH", default = "/var/lib/dhcpcd")]
    path: PathBuf,
}

pub type DhcpcdProvider = NotifyProvider<DhcpcdInterfaceReader>;

impl From<Config> for DhcpcdProvider {
    fn from(Config { path }: Config) -> Self {
        Self::new(path)
    }
}

pub struct DhcpcdInterfaceReader;

#[async_trait]
impl InterfaceReader for DhcpcdInterfaceReader {
    async fn read_interfaces(path: &Path) -> Result<Vec<Interface>> {
        info!("Reading lease files in {:?}", path);

        let lease_files = fs::read_dir(path)?
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| is_lease4_file(path) || is_lease6_file(path));

        let mut interfaces = BTreeMap::new();

        for lease_file in lease_files {
            info!("Reading lease file {:?}", lease_file);

            let name = lease_file
                .file_prefix()
                .ok_or_else(|| anyhow!("Could not get prefix from lease file {:?}", &lease_file))?
                .to_os_string()
                .into_string()
                .map_err(|prefix| {
                    anyhow!(
                        "Could not convert prefix from lease file to string {:?}",
                        prefix
                    )
                })?;

            let interface = interfaces.entry(name.clone()).or_insert(Interface {
                name,
                lease4: Default::default(),
                lease6: Default::default(),
            });

            let bytes = fs::read(&lease_file)?;

            if is_lease4_file(&lease_file) {
                interface.lease4 = Some(decode_v4(&bytes)?);
            } else {
                interface.lease6 = Some(decode_v6(&bytes)?);
            }
        }

        Ok(interfaces.values().cloned().collect())
    }
}

fn decode_v4(bytes: &[u8]) -> Result<Lease4> {
    let mut decoder = v4::Decoder::new(bytes);

    let message = v4::Message::decode(&mut decoder).context("could not decode dhcpv4 message")?;
    let options = message.opts();

    let dns = match options.get(v4::OptionCode::DomainNameServer) {
        Some(v4::DhcpOption::DomainNameServer(domain_name_servers)) => domain_name_servers
            .iter()
            .map(|addr| addr.to_string())
            .collect(),
        _ => vec![],
    };

    let domain = match options.get(v4::OptionCode::DomainName) {
        Some(v4::DhcpOption::DomainName(domain_name)) => Some(domain_name.clone()),
        _ => None,
    };

    Ok(Lease4 { dns, domain })
}

fn decode_v6(bytes: &[u8]) -> Result<Lease6> {
    let mut decoder = v6::Decoder::new(bytes);

    let message = v6::Message::decode(&mut decoder).context("could not decode dhcpv6 message")?;
    let options = message.opts();

    let dns = match options.get(v6::OptionCode::DomainNameServers) {
        Some(v6::DhcpOption::DomainNameServers(domain_name_servers)) => domain_name_servers
            .iter()
            .map(|addr| addr.to_string())
            .collect(),
        _ => vec![],
    };

    let prefix6 = match options.get(v6::OptionCode::IAPD) {
        Some(v6::DhcpOption::IAPD(v6::IAPD { opts, .. })) => {
            match opts.get(v6::OptionCode::IAPrefix) {
                Some(v6::DhcpOption::IAPrefix(v6::IAPrefix {
                    prefix_ip,
                    prefix_len,
                    ..
                })) => vec![Prefix6 {
                    ip: prefix_ip.to_string(),
                    len: *prefix_len as u32,
                }],
                _ => vec![],
            }
        }
        _ => vec![],
    };

    Ok(Lease6 { dns, prefix6 })
}

fn is_lease4_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| matches!(ext.to_str(), Some("lease")))
}

fn is_lease6_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| matches!(ext.to_str(), Some("lease6")))
}
