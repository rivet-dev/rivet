# https://github.com/prometheus/node_exporter/releases
{% set version = '1.4.0' %}

create_node_exporter_user:
  user.present:
    - name: node_exporter
    - shell: /bin/false
    - system: True
    - usergroup: True

install_node_exporter:
  archive.extracted:
    - name: /opt/node_exporter-{{ version }}/
    - source: https://github.com/prometheus/node_exporter/releases/download/v{{ version }}/node_exporter-{{ version }}.linux-amd64.tar.gz
    - source_hash: https://github.com/prometheus/node_exporter/releases/download/v{{ version }}/sha256sums.txt
    - tar_options: z
    - archive_format: tar
  file.managed:
    - name: /usr/bin/node_exporter
    - source: /opt/node_exporter-{{ version }}/node_exporter-{{ version }}.linux-amd64/node_exporter
    - user: node_exporter
    - group: node_exporter
    - mode: 755
    - require:
      - archive: install_node_exporter
  cmd.run:
    - name: node_exporter --version
    - success_stdout: version {{ version }}
    - require:
      - file: install_node_exporter
    - onchanges:
      - file: install_node_exporter

push_node_exporter_service:
  file.managed:
    - name: /etc/systemd/system/node_exporter.service
    - source: salt://node_exporter/files/node_exporter.service

start_node_exporter_service:
  service.running:
    - name: node_exporter
    - enable: True
    - require:
      - file: install_node_exporter
    - watch:
      - file: push_node_exporter_service

