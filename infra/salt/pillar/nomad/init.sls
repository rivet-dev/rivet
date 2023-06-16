{% import_json "/srv/salt-context/rivet/config.json" as rivet %}

nomad:
  leader_count: {{rivet.leader_count}}

