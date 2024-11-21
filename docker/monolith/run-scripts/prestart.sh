#!/bin/sh
cat << EOF
Welcome to Rivet

Configuration:

  Dashboard:       http://127.0.0.1:8080/ui/
  API:             127.0.0.1:8080
  Edge API:        127.0.0.1:8081
  Orchestrator:    127.0.0.1:8082
  Object Storage:  127.0.0.1:9000

Resources:

  Quickstart:      https://rivet.gg/docs/quickstart
  Operation:       https://rivet.gg/docs/self-hosting
  Documentation:   https://rivet.gg/docs
  Discord:         https://rivet.gg/discord

Starting Rivet...
EOF

# Sleep for infinity since this service will be restarted if it exits
sleep infinity

