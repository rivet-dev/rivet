# https://github.com/prometheus/prometheus/releases
{% set version = '2.46.0' %}

{% set pool = grains['rivet']['pool_id'] %}

{% if grains['volumes']['prm']['mount'] %}
{% set device = '/dev/disk/by-id/scsi-0Linode_Volume_' ~ grains['rivet']['name'] ~ '-prm' %}

disk_create_prometheus:
  blockdev.formatted:
    - name: {{device}}
    - fs_type: ext4

disk_mount_prometheus:
  file.directory:
    - name: /mnt/prometheus
    - makedirs: True
  mount.mounted:
    - name: /mnt/prometheus
    - device: {{device}}
    - fstype: ext4
    - require:
      - blockdev: disk_create_prometheus
{% endif %}

create_prometheus_user:
  user.present:
    - name: prometheus
    - shell: /bin/false
    - system: True
    - usergroup: True

mkdir_prometheus:
  file.directory:
    - names:
      - /mnt/prometheus:
        - user: prometheus
        - group: prometheus
        - mode: 700
    - require:
      - user: create_prometheus_user
      {%- if grains['volumes']['prm']['mount'] %}
      - mount: disk_mount_prometheus
      {%- endif %}

install_prometheus:
  archive.extracted:
    - name: /opt/prometheus_{{version}}/
    - source: https://github.com/prometheus/prometheus/releases/download/v{{version}}/prometheus-{{version}}.linux-amd64.tar.gz
    - source_hash: https://github.com/prometheus/prometheus/releases/download/v{{version}}/sha256sums.txt
    - tar_options: z
    - archive_format: tar
  file.managed:
    - names:
      - /usr/bin/prometheus:
        - source: /opt/prometheus_{{ version }}/prometheus-{{ version }}.linux-amd64/prometheus
      - /usr/bin/promtool:
        - source: /opt/prometheus_{{ version }}/prometheus-{{ version }}.linux-amd64/promtool
    - user: prometheus
    - group: prometheus
    - mode: 755
    - require:
      - archive: install_prometheus
  cmd.run:
    - name: prometheus --version
    - success_stdout: version {{ version }}
    - require:
      - file: install_prometheus
    - onchanges:
      - file: install_prometheus

push_etc_prometheus_yaml:
  file.managed:
    - names:
      - /etc/prometheus.yaml:
        - source: salt://prometheus/files/prometheus.yaml
      {%- if pool == "prm-svc" or pool == "local" %}
      - /etc/prometheus.prm-svc.yaml:
        - source: salt://prometheus/files/prometheus.prm-svc.yaml.j2
        - template: jinja
      {%- endif %}
      {%- if pool == "pool-job" or pool == "local" %}
      - /etc/prometheus.prm-job.yaml:
        - source: salt://prometheus/files/prometheus.prm-job.yaml.j2
        - template: jinja
      {%- endif %}

push_prometheus_service:
  file.managed:
    - name: /etc/systemd/system/prometheus.service
    - source: salt://prometheus/files/prometheus.service.j2
    - template: jinja
    - context:
        {%- if pool == "prm-svc" %}
        retention_time: 14d
        {%- else %}
        retention_time: 2d
        {%- endif %}
        retention_size: {{ grains['volumes']['prm']['size']|int * 1000 - 1024 }}MB
        nebula_ipv4: {{ grains['nebula']['ipv4'] }}

start_prometheus_service:
  service.running:
    - name: prometheus
    - enable: True
    - reload: True
    - require:
      - file: install_prometheus
      - file: mkdir_prometheus
      {%- if grains['volumes']['prm']['mount'] %}
      - mount: disk_mount_prometheus
      {%- endif %}
    - watch:
      - file: push_etc_prometheus_yaml
      - file: push_prometheus_service

push_etc_consul_prometheus_hcl:
  file.managed:
    - name: /etc/consul.d/prometheus.hcl
    - source: salt://prometheus/files/consul/prometheus.hcl.j2
    - template: jinja
    - context:
        nebula_ipv4: {{ grains['nebula']['ipv4'] }}
        {%- if pool == "prm-svc" %}
        service_name: prometheus-svc
        {%- elif pool == "prm-job" %}
        service_name: prometheus-job
        {%- elif pool == "local" %}
        service_name: local
        {%- endif %}
    - require:
      - file: create_etc_consul
  cmd.run:
    - name: consul reload
    - require:
      - service: start_consul_service
    - onchanges:
      - file: push_etc_consul_prometheus_hcl

