# TODO: Need to manually initiate after all nodes have clutered:
# cockroach init --insecure --cluster-name rivet-X --host 10.0.8.1:26258

# See https://www.cockroachlabs.com/docs/releases/index.html
{% set version = '23.1.7' %}

{% if grains['volumes']['crdb']['mount'] %}
{% set device = '/dev/disk/by-id/scsi-0Linode_Volume_' ~ grains['rivet']['name'] ~ '-crdb' %}

disk_create_cockroach:
  blockdev.formatted:
    - name: {{ device }}
    - fs_type: ext4

disk_mount_cockroach:
  file.directory:
    - name: /var/lib/cockroach
    - makedirs: True
  mount.mounted:
    - name: /var/lib/cockroach
    - device: {{ device }}
    - fstype: ext4
    - require:
      - blockdev: disk_create_cockroach

{% endif %}

# Synchronize clocks
#
# See https://www.cockroachlabs.com/docs/v22.2/deploy-cockroachdb-on-premises#step-1-synchronize-clocks
sync_clocks_crdb:
  pkg.installed:
    - name: ntp
  file.managed:
    - name: /etc/ntp.conf
    - source: salt://cockroach/files/ntp.conf
    - mode: 644
    - require:
      - pkg: sync_clocks_crdb
  cmd.run:
    - names:
      # Disable NTP if enabled on system by default
      - 'timedatectl set-ntp no':
        - success_retcodes: 0
        - success_stderr: 'Failed to set ntp: NTP not supported'
      - 'systemctl restart ntp': []
      # Validate Google servers added to peers
      - 'ntpq -p':
        - success_stdout: time1.google.co
    - require:
      - file: sync_clocks_crdb
    - onchanges:
      - file: sync_clocks_crdb

create_crdb_user:
  user.present:
    - name: cockroach
    - shell: /bin/false
    - system: True
    - usergroup: True

mkdir_crdb:
  file.directory:
    - names:
      - /var/lib/cockroach:
        - user: cockroach
        - group: cockroach
        - mode: 700
      - /usr/local/lib/cockroach: []
    - require:
      - user: create_crdb_user
      {%- if grains['volumes']['crdb']['mount'] %}
      - mount: disk_mount_cockroach
      {%- endif %}


install_crdb:
  archive.extracted:
    - name: /opt/cockroach_{{ version }}/
    - source: https://binaries.cockroachdb.com/cockroach-v{{version}}.linux-amd64.tgz
    - source_hash: https://binaries.cockroachdb.com/cockroach-v{{version}}.linux-amd64.tgz.sha256sum
    - tar_options: z
    - archive_format: tar
  file.managed:
    - names:
      - /usr/local/bin/cockroach:
        - source: /opt/cockroach_{{ version }}/cockroach-v{{version}}.linux-amd64/cockroach
        - mode: 755
      - /usr/local/lib/cockroach/libgeos.so:
        - source: /opt/cockroach_{{ version }}/cockroach-v{{version}}.linux-amd64/lib/libgeos.so
      - /usr/local/lib/cockroach/libgeos_c.so:
        - source: /opt/cockroach_{{ version }}/cockroach-v{{version}}.linux-amd64/lib/libgeos_c.so
    - require:
      - archive: install_crdb
  cmd.run:
    - names:
      - 'cockroach --version':
        - success_stdout: {{ version }}
    - require:
      - file: install_crdb
    - onchanges:
      - file: install_crdb

push_crdb_service:
  # TODO: Add `start-single-node` if needed
  file.managed:
    - name: /etc/systemd/system/cockroach.service
    - source: salt://cockroach/files/cockroach.service.j2
    - template: jinja
    - context:
        cluster_name: "rivet-{{pillar['rivet']['namespace']}}"
        region: {{grains['rivet']['region_id']}}
        nebula_ipv4: {{ grains['nebula']['ipv4'] }}
        join_node_ips:
          # `exclude_minion` intentionally not specified:
          # > Use the same --join list for all nodes to ensure that the cluster can stabilize.
          # > Source: https://www.cockroachlabs.com/docs/stable/cockroach-start.html#networking
          {%- for server, addr in salt['mine.get']('roles:cockroach', 'nebula_ipv4', tgt_type='grain') | dictsort() %}
          # {{ server }}
          - "{{addr}}:26258"
          {%- endfor %}

start_crdb_service:
  service.running:
  - name: cockroach
  - enable: True
  - require:
    - file: install_crdb
    {%- if grains['volumes']['crdb']['mount'] %}
    - mount: disk_mount_cockroach
    {%- endif %}
    - cmd: sync_clocks_crdb
  - watch:
    - file: push_crdb_service


push_etc_consul_cockroach_hcl:
  file.managed:
    - name: /etc/consul.d/cockroach.hcl
    - source: salt://cockroach/files/consul/cockroach.hcl.j2
    - template: jinja
    - context:
        nebula_ipv4: {{ grains['nebula']['ipv4'] }}
    - require:
      - file: create_etc_consul
  cmd.run:
    - name: consul reload
    - require:
      - service: start_consul_service
      - file: push_etc_consul_cockroach_hcl
    - onchanges:
      - file: push_etc_consul_cockroach_hcl

