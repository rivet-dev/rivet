create_clickhouse_user:
  user.present:
    - name: clickhouse
    - shell: /bin/sh
    - system: True
    - usergroup: True

mkdir_clickhouse:
  file.directory:
    - names:
      - /var/lib/clickhouse:
        - user: clickhouse
        - group: clickhouse
        - mode: 700
      - /var/log/clickhouse-server:
        - user: clickhouse
        - group: clickhouse
        - mode: 700
      - /run/clickhouse-server:
        - user: clickhouse
        - group: clickhouse
        - mode: 700
      - /etc/clickhouse-server:
        - user: clickhouse
        - group: clickhouse
        - mode: 700
    - require:
      - user: create_clickhouse_user

{% if grains['volumes']['ch']['mount'] %}
{% set device = '/dev/disk/by-id/scsi-0Linode_Volume_' ~ grains['rivet']['name'] ~ '-ch' %}

disk_create_clickhouse:
  blockdev.formatted:
    - name: {{ device }}
    - fs_type: ext4

disk_mount_clickhouse:
  mount.mounted:
    - name: /var/lib/clickhouse
    - device: {{ device }}
    - fstype: ext4
    - require:
      - blockdev: disk_create_clickhouse
      - file: mkdir_clickhouse
{% endif %}

# Remove old config directories with residual files
remove_etc_clickhouse_server_dirs:
  file.absent:
    - names:
      - /etc/clickhouse-server/config.d
      - /etc/clickhouse-server/users.d

push_etc_clickhouse_server:
  file.managed:
    - names:
      - /etc/clickhouse-server/config.xml:
        - source: salt://clickhouse/files/clickhouse-server.d/config.xml.j2
      - /etc/clickhouse-server/users.xml:
        - source: salt://clickhouse/files/clickhouse-server.d/users.xml.j2
    - user: clickhouse
    - group: clickhouse
    - template: jinja
    - context:
        nebula_ipv4: {{ grains['nebula']['ipv4'] }}
    - require:
      - user: create_clickhouse_user

push_clickhouse_server_service:
  file.managed:
    - name: /etc/systemd/system/clickhouse-server.service
    - source: salt://clickhouse/files/clickhouse-server.service
    - template: jinja

start_clickhouse_server_service:
  service.running:
    - name: clickhouse-server
    - enable: True
    - reload: True
    - require:
      - cmd: build_nix_shell
      - user: create_clickhouse_user
      {%- if grains['volumes']['ch']['mount'] %}
      - mount: disk_mount_clickhouse
      {%- endif %}
      - file: push_etc_clickhouse_server

push_etc_consul_clickhouse_hcl:
  file.managed:
    - name: /etc/consul.d/clickhouse.hcl
    - source: salt://clickhouse/files/consul/clickhouse.hcl.j2
    - template: jinja
    - context:
        nebula_ipv4: {{ grains['nebula']['ipv4'] }}
    - require:
      - file: create_etc_consul
  cmd.run:
    - name: consul reload
    - require:
      - service: start_consul_service
      - file: push_etc_consul_clickhouse_hcl
    - onchanges:
      - file: push_etc_consul_clickhouse_hcl

