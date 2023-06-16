{% import_json "/srv/salt-context/rivet/config.json" as rivet %}

# Only enable tunnels if Cloudflare Access enabled
{% if rivet.cloudflare is not none and rivet.cloudflare.access is not none %}
{% import_json "/srv/salt-context/terraform/cloudflare_tunnels.json" as cloudflare_tunnels_terraform %}
{% set pool_id = grains['rivet']['pool_id'] %}

{% if pool_id in cloudflare_tunnels_terraform['tunnels'] %}
{% set tunnel = cloudflare_tunnels_terraform['tunnels'][pool_id] %}

cloudflared:
  tunnel_name: {{ tunnel['tunnel_name'] }}
  tunnel_id: {{ tunnel['tunnel_id'] }}
  cert_json: {{ tunnel['cert_json'] | tojson }}
  ingress_json: {{ tunnel['ingress_json'] | tojson | replace("__NEBULA_IPV4__", grains['nebula']['ipv4']) }}

{% endif %}
{% endif %}
