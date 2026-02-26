# DHCPTemplate

`dhcp-template` is a Kubernetes operator written in Rust designed to bridge host-level DHCP
information with Kubernetes resource management.

It allows cluster administrators to automate the creation of network-related resources based on
dynamic data provided by the host's DHCP client.

## Architecture

The project consists of two primary components:

1. **Agent (DaemonSet):** Deployed on selected worker nodes.
   It interfaces with the host's DHCP client to retrieve lease information.
   Currently, only **dhcpcd** is supported.
2. **Operator (Deployment):** Collects data from all active agents and processes it through a template engine.

The operator manages a Custom Resource Definition (CRD) where users define templates.
These templates are rendered and the resulting Kubernetes resources are applied to the cluster.

## Key Features

- **Host-Native DHCP:** Does not implement a private DHCP client; it leverages the existing `dhcpcd` instance on the worker node.
- **Template Rendering:** Uses **Minijinja** for template evaluation, providing full compatibility with Jinja2 syntax.
- **Automated Sync:** Automatically applies rendered resources to the Kubernetes API.
- **Lightweight:** Built in Rust for minimal memory footprint and high performance.

> **Note:** This operator does not currently detect or remediate configuration drift.
> Resources are applied and synced, but external modifications to generated objects are not automatically reverted.

## Example Use Case: Cilium IPv6 Prefix Delegation

A primary use case for `dhcp-template` is managing **Cilium LoadBalancerIPPools** in environments
using IPv6 Prefix Delegation (DHCPv6-PD).

The operator can:

1. Retrieve the delegated prefix from the host via the agent.
2. Render a `CiliumLoadBalancerIPPool` resource using the dynamic prefix.
3. Apply the pool to the cluster so LoadBalancer services can consume the assigned range.

## Usage and Schema

Templates have access to DHCP data structured according to the project's Protobuf definitions.
To understand the available variables and fields for your templates, refer to the schema files located in:

`crates/dhcp-template-api/proto/dhcp-template.proto`

### Template Example

```jinja
apiVersion: k8s.lukasdietrich.com/v1alpha1
kind: DHCPTemplate
metadata:
  name: dhcp-pool-template
spec:
  template: |
    {%- for node in nodes %}
    {%- set lease6 = node.interfaces
                   | map(attribute="lease6")
                   | select("defined")
                   | selectattr("prefix6")
                   | list
    %}
    {%- if lease6 %}
    ---
    apiVersion: cilium.io/v2
    kind: CiliumLoadBalancerIPPool
    metadata:
      name: dhcp-pool-{{ node.name }}
    spec:
      disabled: false
      allowFirstLastIPs: "No"
      blocks:
        {%- for lease in lease6 %}
        {%- for prefix in lease.prefix6 %}
        - cidr: {{ prefix.ip }}/{{ prefix.len }}
        {%- endfor %}
        {%- endfor %}
    {%- endif %}
    {%- endfor %}

```

---

## Installation

Ensure `dhcpcd` is running on your target worker nodes.

```bash
# See https://helm.sh/docs/helm/helm_install/
# You can provide a values file to the install command using `-f values.yaml`

helm install dhcp-template oci://ghcr.io/lukasdietrich/dhcp-template/charts/dhcp-template:${VERSION}
```
