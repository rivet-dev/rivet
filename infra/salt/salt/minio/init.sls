create_minio_user:
  user.present:
    - name: minio-user
    - shell: /bin/false
    - system: True
    - usergroup: True

mkdir_var_lib_minio:
  file.directory:
    - names:
      - /var/lib/minio:
        - user: minio-user
        - group: minio-user
        - mode: 700
    - require:
      - user: create_minio_user

push_etc_default_minio:
  file.managed:
    - name: /etc/default/minio
    - source: salt://minio/files/etc/minio.j2
    - template: jinja

push_minio_server_service:
  file.managed:
    - name: /etc/systemd/system/minio.service
    - source: salt://minio/files/minio.service
    - template: jinja

start_minio_server:
  service.running:
    - name: minio
    - enable: True
    - require:
      - cmd: build_nix_shell
      - file: push_minio_server_service
      - file: mkdir_var_lib_minio
      - file: push_etc_default_minio

push_etc_consul_minio_server_hcl:
  file.managed:
    - name: /etc/consul.d/minio.hcl
    - source: salt://minio/files/consul/minio.hcl.j2
    - template: jinja
    - context:
        nebula_ipv4: {{ grains['nebula']['ipv4'] }}
        domain: {{ pillar['rivet']['domain'] }}
    - require:
      - file: create_etc_consul
  cmd.run:
    - name: consul reload
    - require:
      - service: start_consul_service
      - file: push_etc_consul_minio_server_hcl
    - onchanges:
      - file: push_etc_consul_minio_server_hcl

