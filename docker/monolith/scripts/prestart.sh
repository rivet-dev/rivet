#!/bin/sh
cat << EOF
Starting Rivet

  Version        $(rivet-server --version)
  Operating      https://rivet.gg/docs/self-hosting
  Documentation  https://rivet.gg/docs
  Discord        https://rivet.gg/discord

EOF

# Sleep for infinity since this service will be restarted if it exits
sleep infinity

