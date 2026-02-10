use anyhow::Context;
use dhcp_template_api::{Empty, agent_service_client::AgentServiceClient};
use dhcp_template_crd::DHCPTemplate;
use kube::{Api, Client, api::ListParams};
use log::{LevelFilter, info};
use tonic::Request;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .init();

    let client = Client::try_default()
        .await
        .context("Could not connect to kubernetes")?;

    let templates = Api::<DHCPTemplate>::all(client);
    let params = ListParams::default();

    for template in templates
        .list(&params)
        .await
        .context("Could not fetch templates")?
    {
        info!("bla {:?}", template);
    }

    let mut agent_service = AgentServiceClient::connect("http://[::1]:50051").await?;
    let response = agent_service.get_node(Request::new(Empty {})).await?;
    info!("response {:#?}", response);

    Ok(())
}
