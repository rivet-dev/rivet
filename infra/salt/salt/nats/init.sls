# https://github.com/nats-io/nats-server/releases
{% set version = '2.9.9' %}
{% set exporter_version = '0.10.1' %}

create_nats_user:
  user.present:
    - name: nats
    - shell: /bin/false
    - system: True
    - usergroup: True

install_nats:
  archive.extracted:
    - name: /opt/nats_{{ version }}/
    - source: https://github.com/nats-io/nats-server/releases/download/v{{version}}/nats-server-v{{version}}-linux-amd64.tar.gz
    - source_hash: https://github.com/nats-io/nats-server/releases/download/v{{version}}/SHA256SUMS
    - tar_options: z
    - archive_format: tar
  file.managed:
    - name: /usr/bin/nats-server
    - source: /opt/nats_{{ version }}/nats-server-v{{version}}-linux-amd64/nats-server
    - user: nats
    - group: nats
    - mode: 755
    - require:
      - archive: install_nats

install_nats_exporter:
  archive.extracted:
    - name: /opt/nats_exporter_{{exporter_version}}/
    - source: https://github.com/nats-io/prometheus-nats-exporter/releases/download/v{{exporter_version}}/prometheus-nats-exporter-v{{exporter_version}}-linux-amd64.tar.gz
    - source_hash: https://github.com/nats-io/prometheus-nats-exporter/releases/download/v{{exporter_version}}/prometheus-nats-exporter-v{{exporter_version}}-checksums.txt
    - tar_options: z
    - archive_format: tar
  file.managed:
    - name: /usr/bin/prometheus-nats-exporter
    - source: /opt/nats_exporter_{{exporter_version}}/prometheus-nats-exporter-v{{exporter_version}}-linux-amd64/prometheus-nats-exporter
    - user: nats
    - group: nats
    - mode: 755
    - require:
      - archive: install_nats_exporter

push_etc_nats_server_conf:
  file.managed:
    - name: /etc/nats-server.conf
    - source: salt://nats/files/nats-server.conf.j2
    - mode: 400
    - user: nats
    - group: nats
    - template: jinja
    - context:
        cluster_name: rivet-{{ pillar['rivet']['namespace'] }}
        dc: {{ grains['rivet']['region_id'] }}
        name: {{ grains['rivet']['name'] }}
        nebula_ipv4: {{ grains['nebula']['ipv4'] }}

push_nats_server_service:
  file.managed:
    - name: /etc/systemd/system/nats-server.service
    - source: salt://nats/files/nats-server.service
    - template: jinja

start_nats_server_service:
  service.running:
    - name: nats-server
    - enable: True
    - reload: True
    - require:
      - file: install_nats
    - watch:
      - file: push_etc_nats_server_conf
      - file: push_nats_server_service

push_nats_exporter_service:
  file.managed:
    - name: /etc/systemd/system/nats-exporter.service
    - source: salt://nats/files/nats-exporter.service.j2
    - template: jinja
    - context:
        nebula_ipv4: {{ grains['nebula']['ipv4'] }}

# TODO: Add reload functionality
start_nats_exporter_service:
  service.running:
    - name: nats-exporter
    - enable: True
    - require:
      - file: install_nats_exporter
    - watch:
      - file: push_nats_exporter_service

push_etc_consul_nats_server_hcl:
  file.managed:
    - name: /etc/consul.d/nats_server.hcl
    - source: salt://nats/files/consul/nats_server.hcl.j2
    - template: jinja
    - context:
        nebula_ipv4: {{ grains['nebula']['ipv4'] }}
    - require:
      - file: create_etc_consul
  cmd.run:
    - name: consul reload
    - require:
      - service: start_consul_service
      - file: push_etc_consul_nats_server_hcl
    - onchanges:
      - file: push_etc_consul_nats_server_hcl

