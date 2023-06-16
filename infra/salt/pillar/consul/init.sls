{% import_json "/srv/salt-context/rivet/config.json" as rivet %}

consul:
  leader_count: {{rivet.leader_count}}

