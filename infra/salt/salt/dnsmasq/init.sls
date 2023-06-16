install_dnsmasq:
  ed:
    - name: dnsmasq

push_etc_resolv_conf:
  file.managed:
    - name: /etc/resolv.conf
    - source: salt://dnsmasq/files/resolv.conf
    - require:
      - pkg: dnsmasq

push_etc_dnsmasq:
  file.recurse:
    - name: /etc/dnsmasq.d
    - source: salt://dnsmasq/files/dnsmasq.d
    - file_mode: 644
    - dir_mode: 644
    - clean: True
    - template: jinja
    - require:
      - pkg: dnsmasq

# Stop systemd-resolved since we use dnsmasq to resolve DNS queries
stop_systemd_resolved_service:
  service.dead:
    - name: systemd-resolved
    - enable: False
    - require:
      - file: push_etc_resolv_conf

start_dnsmasq_service:
  service.running:
    - name: dnsmasq
    - enable: True
    - require:
      - file: push_etc_resolv_conf
      - service: stop_systemd_resolved_service
    - watch:
      - pkg: dnsmasq
      - file: push_etc_dnsmasq

