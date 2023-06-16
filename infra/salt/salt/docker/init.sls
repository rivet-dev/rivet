install_docker_core_dependencies:
  ed:
    - pkgs:
      - apt-transport-https
      - ca-certificates
      - curl
      - gnupg2
      - software-properties-common

add_docker_gpg:
  cmd.run:
    - name: 'curl -fsSL https://download.docker.com/linux/debian/gpg | apt-key add -'
    - require:
      - pkg: install_docker_core_dependencies

add_docker_repository:
  pkgrepo.managed:
    - humanname: Docker
    # TODO: $(lsb_release -cs)
    - name: deb [arch=amd64] https://download.docker.com/linux/debian bullseye stable
    - dist: bullseye
    - file: /etc/apt/sources.list.d/docker.list
    - require:
      - cmd: add_docker_gpg

install_docker:
  ed:
    - pkgs:
      - docker-ce
      - docker-ce-cli
      - containerd.io
    - refresh: True
    - require:
      - pkgrepo: add_docker_repository

check_docker:
  cmd.run:
    - name: docker run hello-world
    - require:
      - pkg: install_docker
    - onchanges:
      - pkg: install_docker

