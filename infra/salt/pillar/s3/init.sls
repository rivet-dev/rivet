{% import_json "/srv/salt-context/rivet/config.json" as rivet %}
{% import_json "/srv/salt-context/rivet/secrets.json" as rivet_secrets %}

s3:
  endpoint_internal: {{ rivet.s3.endpoint_internal }}
  endpoint_external: {{ rivet.s3.endpoint_external }}
  region: {{ rivet.s3.region }}
  access_key_id: {{ rivet_secrets.s3.persistent_access_key_id }}
  access_key_secret: {{ rivet_secrets.s3.persistent_access_key_secret }}

