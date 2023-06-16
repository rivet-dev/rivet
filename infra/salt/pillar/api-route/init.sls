{% import_json "/srv/salt-context/rivet/secrets.json" as rivet_secrets %}

rivet:
  api-route: {{ rivet_secrets['rivet']['api_route'] | yaml }}

