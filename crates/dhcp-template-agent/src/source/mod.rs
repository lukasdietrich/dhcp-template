use std::fmt::Debug;

use anyhow::Result;
use async_trait::async_trait;
use dhcp_template_api::Interface;

pub mod dhcpcd;

#[async_trait]
pub trait Source: Debug + Sync + Send {
    async fn get_node(&self) -> Result<Vec<Interface>>;
}
