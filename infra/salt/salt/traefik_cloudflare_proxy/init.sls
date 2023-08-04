create_traefik_cloudflare_proxy_user:
  user.present:
    - name: traefik_cloudflare_proxy
    - shell: /bin/false
    - system: True
    - usergroup: True

create_etc_traefik_cloudflare_proxy:
  file.directory:
    - names:
      - /etc/traefik_cloudflare_proxy/: {}
      - /etc/traefik_cloudflare_proxy/dynamic/: {}
    - user: traefik_cloudflare_proxy
    - group: traefik_cloudflare_proxy
    - mode: 550

push_etc_traefik_cloudflare_proxy:
  file.managed:
    - names:
      # Static config
      - /etc/traefik_cloudflare_proxy/traefik.toml:
        - source: salt://traefik_cloudflare_proxy/files/traefik.toml.j2

      # Dynamic configs
      - /etc/traefik_cloudflare_proxy/dynamic/common.toml:
        - source: salt://traefik_cloudflare_proxy/files/dynamic/common.toml.j2

    - user: traefik_cloudflare_proxy
    - group: traefik_cloudflare_proxy
    - template: jinja
    - context:
        loki_url: "{{ pillar['rivet']['logging']['loki']['endpoint'] }}"
        access_client_id: "{{ pillar['cloudflare']['access']['proxy']['client_id'] }}"
        access_client_secret: "{{ pillar['cloudflare']['access']['proxy']['client_secret'] }}"
    - mode: 440
    - require:
      - file: create_etc_traefik_cloudflare_proxy

push_traefik_cloudflare_proxy_service:
  file.managed:
    - name: /etc/systemd/system/traefik_cloudflare_proxy.service
    - source: salt://traefik_cloudflare_proxy/files/traefik_cloudflare_proxy.service

# Manually restart the Traefik service yourself in order to prevent terminating
# connections needlessly
start_traefik_cloudflare_proxy_service:
  service.running:
    - name: traefik_cloudflare_proxy
    - enable: True
    - require:
      - file: push_traefik_cloudflare_proxy_service
      - file: push_etc_traefik_cloudflare_proxy
