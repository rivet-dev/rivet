# https://github.com/containernetworking/plugins/releases
{% set version = '1.3.0' %}

install_cni_plugins:
  archive.extracted:
    - name: /opt/cni-plugins-{{ version }}/
    - source: https://github.com/containernetworking/plugins/releases/download/v{{ version }}/cni-plugins-linux-amd64-v{{ version }}.tgz
    - source_hash: https://github.com/containernetworking/plugins/releases/download/v{{ version }}/cni-plugins-linux-amd64-v{{ version }}.tgz.sha512
    - tar_options: z
    - archive_format: tar
  file.copy:
    - name: /opt/cni/bin
    - source: /opt/cni-plugins-{{ version }}
    - force: True
    - makedirs: True
    - require:
      - archive: install_cni_plugins
    - onchanges:
      - archive: install_cni_plugins

