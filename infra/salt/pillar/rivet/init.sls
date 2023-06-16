{% import_json "/srv/salt-context/rivet/config.json" as rivet %}

rivet: {{rivet|yaml}}

