# Nebula is installed in Terraform since we need Nebula to be running before we
# can do anything else

{% if 'consul-client' in grains['roles'] %}
push_etc_consul_nebula_hcl:
  file.managed:
    - name: /etc/consul.d/nebula.hcl
    - source: salt://nebula/files/consul/nebula.hcl.j2
    - template: jinja
    - require:
      - file: create_etc_consul
  cmd.run:
    - name: consul reload
    - require:
      - service: start_consul_service
      - file: push_etc_consul_nebula_hcl
    - onchanges:
      - file: push_etc_consul_nebula_hcl
{% endif %}

