create_cloudflare_proxy_user:
  user.present:
    - name: cloudflare_proxy
    - shell: /bin/false
    - system: True
    - usergroup: True

create_etc_cloudflare_proxy:
  file.directory:
    - names:
      - /etc/cloudflare_proxy/: {}
      - /etc/cloudflare_proxy/dynamic/: {}
    - user: cloudflare_proxy
    - group: cloudflare_proxy
    - mode: 550
    - require:
      - user: create_cloudflare_proxy_user

push_etc_cloudflare_proxy:
  file.managed:
    - names:
      # Static config
      - /etc/cloudflare_proxy/traefik.toml:
        - source: salt://cloudflare_proxy/files/traefik.toml.j2

      # Dynamic configs
      - /etc/cloudflare_proxy/dynamic/common.toml:
        - source: salt://cloudflare_proxy/files/dynamic/common.toml.j2

    - user: cloudflare_proxy
    - group: cloudflare_proxy
    - template: jinja
    - context:
        loki_url: "{{ pillar['rivet']['logging']['loki']['endpoint'] }}"
        access_client_id: "{{ pillar['cloudflare']['access']['proxy']['client_id'] }}"
        access_client_secret: "{{ pillar['cloudflare']['access']['proxy']['client_secret'] }}"
    - mode: 440
    - require:
      - file: create_etc_cloudflare_proxy

push_cloudflare_proxy_service:
  file.managed:
    - name: /etc/systemd/system/cloudflare-proxy.service
    - source: salt://cloudflare_proxy/files/cloudflare-proxy.service

# Manually restart the Traefik service yourself in order to prevent terminating
# connections needlessly
start_cloudflare_proxy_service:
  service.running:
    - name: cloudflare-proxy
    - enable: True
    - require:
      - file: push_cloudflare_proxy_service
      - file: push_etc_cloudflare_proxy

{%- if 'consul-client' in grains['roles'] %}
push_etc_consul_cloudflare_proxy_hcl:
  file.managed:
    - name: /etc/consul.d/cloudflare-proxy.hcl
    - source: salt://cloudflare_proxy/files/consul/cloudflare-proxy.hcl
    - template: jinja
    - require:
      - file: create_etc_consul
  cmd.run:
    - name: consul reload
    - require:
      - service: start_consul_service
      - file: push_etc_consul_cloudflare_proxy_hcl
    - onchanges:
      - file: push_etc_consul_cloudflare_proxy_hcl
{%- endif %}

