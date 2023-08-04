install_plugin_loki:
  cmd.run:
    - name: docker plugin install grafana/loki-docker-driver:latest --alias loki --grant-all-permissions
    - unless: docker plugin list | grep loki
