# https://github.com/grafana/loki/releases & https://hub.docker.com/r/grafana/loki-docker-driver/tags
{% set version = '2.8.2' %}

install_plugin_loki:
  cmd.run:
    - name: docker plugin install grafana/loki-docker-driver:{{version}} --alias loki --grant-all-permissions
    - unless: docker plugin list | grep loki
