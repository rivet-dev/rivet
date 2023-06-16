# https://github.com/traefik/traefik/releases
{% set version = '2.9.6' %}

{% set pool = grains['rivet']['pool_id'] %}
{% if pool == 'ing-px' or pool == 'local' %}
  {% set tls_certs = ['cloudflare_rivet_gg'] %}
  {% set tls_client_cert = true %}
{% elif pool == 'ing-job' %}
  {% set tls_certs = ['letsencrypt_rivet_job'] %}
  {% set tls_client_cert = false %}
{% endif %}

# See on-prem installation instructions: https://doc.traefik.io/traefik-enterprise/installing/on-premise/#systemd-linux-only

create_traefik_exporter_user:
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
      - archive: install_traefik
  cmd.run:
    - name: traefik version
    - success_stdout: '{{ version }}'
    - require:
      - file: install_traefik
    - onchanges:
      - file: install_traefik

create_etc_traefik:
  file.directory:
    - names:
      - /etc/traefik/: {}
      - /etc/traefik/dynamic/: {}
      - /etc/traefik/tls/: {}
      - /opt/traefik/: {}
    - user: traefik
    - group: traefik
    - mode: 550

push_etc_traefik:
  file.managed:
    - names:
      # Static config
      - /etc/traefik/traefik.toml:
        - source: salt://traefik/files/traefik.toml.j2

      # Dynamic configs
      - /etc/traefik/dynamic/common.toml:
        - source: salt://traefik/files/dynamic/common.toml.j2
      - /etc/traefik/dynamic/tls.toml:
        - source: salt://traefik/files/dynamic/tls.toml.j2

      # TLS certs
      {%- for cert_id in tls_certs %}
      - /etc/traefik/tls/{{ cert_id }}_cert.pem:
        - contents: |
            {{ pillar['tls']['tls_certs'][cert_id]['cert_pem'] | indent(12) }}
      - /etc/traefik/tls/{{ cert_id }}_key.pem:
        - contents: |
            {{ pillar['tls']['tls_certs'][cert_id]['key_pem'] | indent(12) }}
      {%- endfor %}

      # TLS client cert
      {%- if tls_client_cert %}
      - /etc/traefik/tls/traefik_client.crt:
        - source: https://developers.cloudflare.com/ssl/static/authenticated_origin_pull_ca.pem
        - source_hash: c1a581133a27f5fc98cbb1f32d048925470deab627e6b5f1b586f6df075d8493ec7d08ede04d0f31ec2c2cd74de27ed0df866e3874ad54a9e962695759ba7e5b
      {%- endif %}

    - user: traefik
    - group: traefik
    - template: jinja
    - context:
        provider_http:
          endpoint: 'https://route.api.{{ pillar['rivet']['domain']['base'] }}/v1/traefik/config?region={{grains['rivet']['region_id']}}&pool={{grains['rivet']['pool_id']}}&token={{pillar['rivet']['api-route']['token']}}'
          {%- if pool == 'ing-px' %}
          poll_interval: '2.5s'
          {%- else %}
          poll_interval: '1s'
          {%- endif %}
        namespace: {{ pillar['rivet']['namespace'] }}
        pool: {{ pool }}
        domain: {{ pillar['rivet']['domain'] }}
        nebula_ipv4: {{ grains['nebula']['ipv4'] }}
        traefik:
          tls_certs: {{ tls_certs }}
          tls_client_cert: {{ tls_client_cert }}
    - mode: 440
    - require:
      - file: create_etc_traefik


push_traefik_service:
  file.managed:
    - name: /etc/systemd/system/traefik.service
    - source: salt://traefik/files/traefik.service

# Manually restart the Traefik service yourself in order to prevent terminating
# connections needlessly
start_traefik_service:
  service.running:
    - name: traefik
    - enable: True
    - require:
      - file: install_traefik
      - file: push_traefik_service
      - file: push_etc_traefik

{%- if 'consul-client' in grains['roles'] %}
push_etc_consul_traefik_hcl:
  file.managed:
    - name: /etc/consul.d/traefik.hcl
    - source: salt://traefik/files/consul/traefik.hcl.j2
    - template: jinja
    - context:
        service_name: {{ pool }}
        nebula_ipv4: {{ grains['nebula']['ipv4'] }}
    - require:
      - file: create_etc_consul
  cmd.run:
    - name: consul reload
    - require:
      - service: start_consul_service
      - file: push_etc_consul_traefik_hcl
    - onchanges:
      - file: push_etc_consul_traefik_hcl
{%- endif %}

