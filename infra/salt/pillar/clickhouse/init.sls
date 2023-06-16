{% import_json "/srv/salt-context/rivet/secrets.json" as rivet_secrets %}

clickhouse: {{ rivet_secrets['clickhouse'] | yaml }}

