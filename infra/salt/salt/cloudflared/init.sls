# https://github.com/cloudflare/cloudflared/releases
{% set version = '2022.11.1' %}
{% set pool_id = grains['rivet']['pool_id'] %}

{% if 'cloudflared' in pillar %}

create_etc_cloudflared:
  file.directory:
    - name: /etc/cloudflared
    - mode: 700

push_etc_cloudflared:
  file.managed:
    - names:
      # Must use `.yml` instead of `.yaml` because Cloudflare may try to
      # overwrite the file. Unable to reproduce this bug consistently.
      - /etc/cloudflared/config.yml:
        - source: salt://cloudflared/files/config.yml.j2
      - /etc/cloudflared/cert.json:
        - contents: |
            {{ pillar['cloudflared']['cert_json'] }}
    - template: jinja

push_cloudflared_service:
  file.managed:
    - name: /etc/systemd/system/cloudflared.service
    - source: salt://cloudflared/files/cloudflared.service

start_cloudflared_service:
  service.running:
    - name: cloudflared
    - enable: True
    - reload: True
    - require:
      - cmd: build_nix_shell
      - file: push_etc_cloudflared
      - file: push_cloudflared_service

{% else %}

# Disable any residual default cloudflared service
disable_cloudflared_service:
  service.dead:
    - name: cloudflared
    - enable: False

{% endif %}
