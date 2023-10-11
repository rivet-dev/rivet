{% set pool = grains['rivet']['pool_id'] %}
{% if pool == 'ing-px' or pool == 'local' %}
  {% set tls_certs = ['cloudflare_rivet_gg'] %}
  {% set tls_client_cert = true %}
{% elif pool == 'ing-job' %}
  {% set tls_certs = ['letsencrypt_rivet_job'] %}
  {% set tls_client_cert = false %}
{% endif %}

# See on-prem installation instructions: https://doc.traefik.io/traefik-enterprise/installing/on-premise/#systemd-linux-only

create_ingress_proxy_user:
  user.present:
    - name: ingress_proxy
    - shell: /bin/false
    - system: True
    - usergroup: True

create_etc_ingress_proxy:
  file.directory:
    - names:
      - /etc/ingress_proxy/: {}
      - /etc/ingress_proxy/dynamic/: {}
      - /etc/ingress_proxy/tls/: {}
      - /opt/ingress_proxy/: {}
    - user: ingress_proxy
    - group: ingress_proxy
    - mode: 550
    - require:
      - user: create_ingress_proxy_user

push_etc_ingress_proxy:
  file.managed:
    - names:
      # Static config
      - /etc/ingress_proxy/traefik.toml:
        - source: salt://ingress_proxy/files/traefik.toml.j2

      # Dynamic configs
      - /etc/ingress_proxy/dynamic/common.toml:
        - source: salt://ingress_proxy/files/dynamic/common.toml.j2
      - /etc/ingress_proxy/dynamic/tls.toml:
        - source: salt://ingress_proxy/files/dynamic/tls.toml.j2

      # TLS certs
      {%- for cert_id in tls_certs %}
      - /etc/ingress_proxy/tls/{{ cert_id }}_cert.pem:
        - contents: |
            {{ pillar['tls']['tls_certs'][cert_id]['cert_pem'] | indent(12) }}
      - /etc/ingress_proxy/tls/{{ cert_id }}_key.pem:
        - contents: |
            {{ pillar['tls']['tls_certs'][cert_id]['key_pem'] | indent(12) }}
      {%- endfor %}

      # TLS client cert
      {%- if tls_client_cert %}
      - /etc/ingress_proxy/tls/traefik_client.crt:
        - source: https://developers.cloudflare.com/ssl/static/authenticated_origin_pull_ca.pem
        - source_hash: c1a581133a27f5fc98cbb1f32d048925470deab627e6b5f1b586f6df075d8493ec7d08ede04d0f31ec2c2cd74de27ed0df866e3874ad54a9e962695759ba7e5b
      {%- endif %}

    - user: ingress_proxy
    - group: ingress_proxy
    - template: jinja
    - context:
        provider_http:
          endpoint: 'https://route.api.{{ pillar['rivet']['domain']['base'] }}/v1/traefik/config?region={{grains['rivet']['region_id']}}&pool={{grains['rivet']['pool_id']}}&token={{pillar['rivet']['api-route']['token']}}'
          {%- if pool == 'ing-px' %}
          poll_interval: '2.5s'
          {%- else %}
          poll_interval: '0.25s'
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
      - file: create_etc_ingress_proxy

push_ingress_proxy_service:
  file.managed:
    - name: /etc/systemd/system/ingress-proxy.service
    - source: salt://ingress_proxy/files/ingress-proxy.service
    - template: jinja

# Stop and disable the old service under the old name first
stop_traefik_service:
  service.dead:
    - name: traefik
    - enable: False

# Manually restart the Traefik service yourself in order to prevent terminating
# connections needlessly
start_ingress_proxy_service:
  service.running:
    - name: ingress-proxy
    - enable: True
    - require:
      - file: install_traefik
      - file: push_ingress_proxy_service
      - file: push_etc_ingress_proxy
      - service: stop_traefik_service

{%- if 'consul-client' in grains['roles'] %}
push_etc_consul_ingress_proxy_hcl:
  file.managed:
    - name: /etc/consul.d/ingress-proxy.hcl
    - source: salt://ingress_proxy/files/consul/ingress-proxy.hcl.j2
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
      - file: push_etc_consul_ingress_proxy_hcl
    - onchanges:
      - file: push_etc_consul_ingress_proxy_hcl
{%- endif %}

