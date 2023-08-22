{% import_json "/srv/salt-context/rivet/config.json" as rivet %}
{% import_json "/srv/salt-context/rivet/secrets.json" as rivet_secrets %}

s3:
  config: {{ rivet.s3 }}
  access: {{ rivet_secrets.s3 }}
