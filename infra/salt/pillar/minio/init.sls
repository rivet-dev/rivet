{% import_json "/srv/salt-context/rivet/secrets.json" as rivet_secrets %}

minio: {{ rivet_secrets['minio'] | yaml }}

