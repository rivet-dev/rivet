install_sysctl:
  file.managed:
    - name: /etc/sysctl.d/10-rivet.conf
    - source: salt://sysctl/files/rivet.conf
    - user: root
    - group: root
    - mode: 644

# TODO: Run `sysctl -p /etc/sysctl.d/10-rivet.conf` after updating this file

