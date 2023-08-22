{% import_json "/srv/salt-context/rivet/secrets.json" as rivet_secrets %}

cloudflare: {{ rivet_secrets.cloudflare }}
