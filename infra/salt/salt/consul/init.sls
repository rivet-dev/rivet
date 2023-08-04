# https://github.com/hashicorp/consul/releases
{% set version = '1.16.0' %}

create_consul_user:
  user.present:
    - name: consul
    - shell: /bin/false
    - home: /etc/consul.d
    - createhome: False
    - system: True
    - usergroup: True

install_consul:
  archive.extracted:
    - name: /opt/consul-{{ version }}/
    - source: https://releases.hashicorp.com/consul/{{ version }}/consul_{{ version }}_linux_amd64.zip
    - source_hash: https://releases.hashicorp.com/consul/{{ version }}/consul_{{ version }}_SHA256SUMS
    - archive_format: zip
    - enforce_toplevel: False
  file.managed:
    - name: /usr/bin/consul
    - source: /opt/consul-{{ version }}/consul
    - user: root
    - group: root
    - mode: 755
    - require:
      - archive: install_consul
  cmd.run:
    - name: consul version
    - success_stdout: Consul v{{ version }}
    - onchanges:
      - file: install_consul
    - require:
      - file: install_consul

create_opt_consul:
  file.directory:
    - name: /opt/consul/data
    - makedirs: True
    - user: consul
    - group: consul
    - mode: 700
    - require:
      - user: create_consul_user

create_etc_consul:
  file.directory:
    - name: /etc/consul.d
    - user: consul
    - group: consul
    - mode: 700
    - require:
      - user: create_consul_user

push_etc_consul:
  file.managed:
    - names: 
      - /etc/consul.d/common.hcl:
        - source: salt://consul/files/consul.d/common.hcl.j2
      {%- if 'consul-server' in grains['roles'] %}
      - /etc/consul.d/server.hcl:
        - source: salt://consul/files/consul.d/server.hcl.j2
      {%- endif %}
    - user: consul
    - group: consul
    - template: jinja
    - mode: 640
    - require:
      - file: create_etc_consul
  cmd.run:
    - name: consul validate /etc/consul.d/
    - require:
      - file: push_etc_consul
    - onchanges:
      - file: push_etc_consul

push_consul_service:
  file.managed:
    - name: /etc/systemd/system/consul.service
    - source: salt://consul/files/consul.service
    - template: jinja

start_consul_service:
  service.running:
    - name: consul
    - enable: True
    - reload: True
    - require:
      - file: create_opt_consul
      - file: push_etc_consul
      - file: push_consul_service
    - watch:
      - file: install_consul
      - file: push_etc_consul
      - file: push_consul_service

