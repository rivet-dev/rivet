{% set version = '7.0.8' %}


{% set pool = pillar['rivet']['pools'][grains['rivet']['pool_id']] %}
{% set dbs = pool['redis_dbs'] %}

create_redis_user:
  user.present:
    - name: redis
    - shell: /bin/false
    - system: True
    - usergroup: True


{% for db in pool['redis_dbs'] %}
{% set db_config = pillar['rivet']['redis']['dbs'][db] %}
{% set suffix = db | replace('-', '_') %}

{% if db_config['persistent'] %}
create_var_lib_redis_{{suffix}}:
  file.directory:
    - name: /var/lib/redis/{{db}}
    - makedirs: True

create_var_lib_redis_redis_{{suffix}}:
  file.directory:
    - name: /var/lib/redis/{{db}}/db
    - user: redis
    - group: redis
    - mode: 700
    - require:
      - file: create_var_lib_redis_{{suffix}}
      - user: create_redis_user
{% endif %}

create_etc_redis_{{suffix}}:
  file.directory:
    - name: /etc/redis
    - mode: 700

push_etc_redis_{{suffix}}:
  file.managed:
    - name: /etc/redis/{{db}}.conf
    - source: salt://redis/files/redis.conf.j2
    - template: jinja
    - context:
        persistent: {{ db_config['persistent'] }}
        nebula_ipv4: {{ grains['nebula']['ipv4'] }}
        maxmemory_mb: {{ grains['mem_total']|int - 250 }}
        db: {{ db }}
        port: {{ db_config['port'] }}

push_redis_service_{{suffix}}:
  file.managed:
    - name: /etc/systemd/system/{{db}}.service
    - source: salt://redis/files/redis.service.j2
    - template: jinja
    - context:
        version: {{ version }}
        db: {{ db }}
        port: {{ db_config['port'] }}
        persistent: {{ db_config['persistent'] }}

start_redis_service_{{suffix}}:
  service.running:
    - name: {{ db }}
    - enable: True
    - require:
      - pkg: install_docker
      {%- if db_config['persistent'] %}
      - file: create_var_lib_redis_redis_{{suffix}}
      {%- endif %}
      - file: push_etc_redis_{{suffix}}

push_etc_consul_redis_hcl_{{suffix}}:
  file.managed:
    - name: /etc/consul.d/{{db}}.hcl
    - source: salt://redis/files/consul/redis.hcl.j2
    - makedirs: True
    - template: jinja
    - context:
        service_name: {{ db }}
        port: {{ db_config['port'] }}
    - require:
      - file: create_etc_consul
  cmd.run:
    - name: consul reload
    - require:
      - service: start_consul_service
      - file: push_etc_consul_redis_hcl_{{suffix}}
    - onchanges:
      - file: push_etc_consul_redis_hcl_{{suffix}}

{% endfor %}

