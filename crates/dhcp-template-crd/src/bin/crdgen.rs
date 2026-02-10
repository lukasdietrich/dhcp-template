use dhcp_template_crd::DHCPTemplate;
use kube::CustomResourceExt;

fn main() -> anyhow::Result<()> {
    println!("{}", to_yaml::<DHCPTemplate>()?);
    Ok(())
}

fn to_yaml<C>() -> anyhow::Result<String>
where
    C: CustomResourceExt,
{
    let crd = C::crd();
    Ok(serde_yaml::to_string(&crd)?)
}
