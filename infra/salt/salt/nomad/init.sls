# https://github.com/hashicorp/nomad/releases
{% set version = '1.6.1' %}
{% set create_nomad_user = 'nomad-server' in grains['roles'] %}

{% if create_nomad_user %}
# Create Nomad user forl the server to run with. Clients will run with root.
create_nomad_user:
  user.present:
    - name: nomad
    - shell: /bin/false
    - home: /etc/nomad.d
    - createhome: False
    - system: True
    - usergroup: True
{% endif %}

install_nomad:
  archive.extracted:
    - name: /opt/nomad-{{ version }}/
    - source: https://releases.hashicorp.com/nomad/{{ version }}/nomad_{{ version }}_linux_amd64.zip
    - source_hash: https://releases.hashicorp.com/nomad/{{ version }}/nomad_{{ version }}_SHA256SUMS
    - archive_format: zip
    - enforce_toplevel: False
  file.managed:
    - name: /usr/local/bin/nomad
    - source: /opt/nomad-{{ version }}/nomad
    - user: root
    - group: root
    - mode: 755
    - require:
      - archive: install_nomad
  cmd.run:
    - name: nomad version
    - success_stdout: Nomad v{{ version }}
    - require:
      - file: install_nomad
    - onchanges:
      - file: install_nomad

create_opt_nomad:
  file.directory:
    - name: /opt/nomad/data
    - makedirs: True
    {%- if create_nomad_user %}
    - user: nomad
    - group: nomad
    {% endif %}
    - mode: 700

{% if 'nomad-client' in grains['roles'] %}
create_opt_dirs:
  file.directory:
    - names:
      - /opt/vector/data: []
    - makedirs: True
    - mode: 700
{% endif %}

create_etc_nomad:
  file.directory:
    - name: /etc/nomad.d
    {%- if create_nomad_user %}
    - user: nomad
    - group: nomad
    {%- endif %}
    - mode: 700
    - makedirs: True

push_etc_nomad:
  file.managed:
    - names: 
      - /etc/nomad.d/common.hcl:
        - source: salt://nomad/files/nomad.d/common.hcl.j2
      {%- if 'nomad-server' in grains['roles'] %}
      - /etc/nomad.d/server.hcl:
        - source: salt://nomad/files/nomad.d/server.hcl.j2
      {%- endif %}
      {%- if 'nomad-client' in grains['roles'] %}
      - /etc/nomad.d/client.hcl:
        - source: salt://nomad/files/nomad.d/client.hcl.j2
      {%- endif %}
    {%- if create_nomad_user %}
    - user: nomad
    - group: nomad
    {%- endif %}
    - template: jinja
    - mode: 640
    - context:
        nebula_ipv4: {{ grains['nebula']['ipv4'] }}
    - require:
      - file: create_etc_nomad

push_nomad_service:
  file.managed:
    - name: /etc/systemd/system/nomad.service
    - source: salt://nomad/files/nomad.service.j2
    - template: jinja

start_nomad_service:
  service.running:
    - name: nomad
    - enable: True
    - reload: True
    - require:
      - file: create_opt_nomad
      - file: push_etc_nomad
      {%- if 'nomad-client' in grains['roles'] %}
      - file: create_opt_dirs
      {%- if 'docker' in grains['roles'] %}
      - file: install_cni_plugins
      - pkg: install_docker
      {%- endif %}
      {%- endif %}
    - watch:
      - file: install_nomad
      - file: push_etc_nomad
      - file: push_nomad_service

