update_pkgs:
  pkg.uptodate: []

install_common_pkgs:
  pkg.installed:
    - pkgs:
      - apt-transport-https
      - ca-certificates
      - software-properties-common
      - curl
      - jq

