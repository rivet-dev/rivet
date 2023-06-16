nix:
  dependencies:
    {%- if 'rivet' in grains %}
    - cloudflared
    {%- endif %}
    {%- if 'clickhouse' in grains['roles'] %}
    - clickhouse
    {%- endif %}
    {%- if 'minio' in grains['roles'] %}
    - minio
    {%- endif %}
    {%- if 'traffic-server' in grains['roles'] %}
    - traffic_server
    {%- endif %}

