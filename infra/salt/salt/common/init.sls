update_pkgs:
  pkg.uptodate: []

install_common_pkgs:
  ed:
    - pkgs:
      - apt-transport-https
      - ca-certificates
      - software-properties-common
      - curl
      - jq

