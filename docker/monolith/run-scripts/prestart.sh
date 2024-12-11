#!/bin/sh
cat << EOF
Welcome to Rivet

Configuration:

  Rivet Server:

    Dashboard:        http://127.0.0.1:8080/ui/
    API:              127.0.0.1:8080
    Edge API:         127.0.0.1:8081
    Orchestrator:     127.0.0.1:8082
    Object Storage:   127.0.0.1:9000

  Rivet Guard:

    HTTP:             127.0.0.1:7080
    HTTPS:            127.0.0.1:7443
    TCP & UDP:        127.0.0.1:7500-7599

  Rivet Client:

    Host Networking:  127.0.0.1:7600-7699

Resources:

  Quickstart:         https://rivet.gg/docs/quickstart
  Operation:          https://rivet.gg/docs/self-hosting
  Documentation:      https://rivet.gg/docs
  Discord:            https://rivet.gg/discord

Starting Rivet...
EOF

# Sleep for infinity since this service will be restarted if it exits
sleep infinity

