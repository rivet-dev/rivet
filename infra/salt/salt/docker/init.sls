install_docker_core_dependencies:
  pkg.installed:
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
  pkg.installed:
    - pkgs:
      - docker-ce
      - docker-ce-cli
      - containerd.io
    - refresh: True
    - require:
      - pkgrepo: add_docker_repository

create_docker_user:
  user.present:
    - name: docker-salt
    - shell: /bin/false
    - system: True
    - usergroup: True

create_etc_docker:
  file.directory:
    - names:
      - /etc/docker/: {}
    - user: docker-salt
    - group: docker-salt
    - mode: 550

push_etc_docker:
  file.managed:
    - names:
      - /etc/docker/daemon.json:
        - source: salt://docker/files/daemon.json
    - user: docker-salt
    - group: docker-salt
    - mode: 440
    - require:
      - file: create_etc_docker

check_docker:
  cmd.run:
    - name: docker run hello-world
    - require:
      - pkg: install_docker
      - file: push_etc_docker
    - onchanges:
      - pkg: install_docker

