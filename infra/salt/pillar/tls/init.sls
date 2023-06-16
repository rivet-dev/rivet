{% import_json "/srv/salt-context/terraform/tls.json" as tls %}

tls: {{tls|yaml}}

