install_nix:
  cmd.run:
    - name: curl --proto '=https' --tlsv1.2 -sSf -L https://install.determinate.systems/nix | sh -s -- install --no-confirm
    - unless:
      - test -f /root/.nix-profile/bin/nix

copy_nix_shell:
  file.recurse:
    - name: /var/rivet-nix/source
    - source: salt://nix/files/source
    - include_empty: True
    - file_mode: 755
    - dir_mode: 755
    - clean: True

build_nix_shell:
  cmd.script:
    - name: salt://nix/files/build.sh.j2
    - template: jinja
    - require:
      - file: copy_nix_shell

