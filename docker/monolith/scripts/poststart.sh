#!/bin/sh
cat << EOF
Server is ready

  Dashboard      http://127.0.0.1:8080/ui/
  API            127.0.0.1:8081
  Edge API       127.0.0.1:8081
  Orchestrator   127.0.0.1:8082
  S3             127.0.0.1:9000
  Server Config  /etc/rivet-server/config.yaml
  Client Config  /etc/rivet-server/config.yaml

Please visit https://rivet-gg/docs/quickstart to get started.
EOF

# Sleep for infinity since this service will be restarted if it exits
sleep infinity

