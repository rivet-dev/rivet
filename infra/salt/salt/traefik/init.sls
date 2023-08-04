# https://github.com/traefik/traefik/releases
{% set version = '2.10.4' %}

create_traefik_user:
  user.present:
    - name: traefik
    - shell: /bin/false
    - system: True
    - usergroup: True

install_traefik:
  archive.extracted:
    - name: /opt/traefik-{{ version }}
    - source: https://github.com/traefik/traefik/releases/download/v{{ version }}/traefik_v{{ version }}_linux_amd64.tar.gz
    - source_hash: https://github.com/traefik/traefik/releases/download/v{{ version }}/traefik_v{{ version }}_checksums.txt
    - tar_options: z
    - archive_format: tar
    - enforce_toplevel: False
  file.managed:
    - name: /usr/bin/traefik
    - source: /opt/traefik-{{ version }}/traefik
    - user: traefik
    - group: traefik
    - mode: 755
    - require:
      - user: create_traefik_user
      - archive: install_traefik
  cmd.run:
    - name: traefik version
    - success_stdout: '{{ version }}'
    - require:
      - file: install_traefik
    - onchanges:
      - file: install_traefik

# Delete old Treafik Consul file that was moved
remove_etc_consul_traefik_hcl:
  file.absent:
    - name: /etc/consul.d/traefik.hcl
  cmd.run:
    - name: consul reload
    - require:
      - service: start_consul_service
      - file: remove_etc_consul_traefik_hcl
    - onchanges:
      - file: remove_etc_consul_traefik_hcl

