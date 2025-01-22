#!/bin/sh

# TODO: build image
# TODO: connect image

docker run -it --rm --pull=always \
    -e SANDBOX_RUNTIME_CONTAINER_IMAGE=docker.all-hands.dev/all-hands-ai/runtime:0.20-nikolaik \
    -e LOG_ALL_EVENTS=true \
    -v /var/run/docker.sock:/var/run/docker.sock \
    -v ~/.openhands-state:/.openhands-state \
    -p 3500:3000 \
    --add-host host.docker.internal:host-gateway \
    --name rivet-openhands \
    docker.all-hands.dev/all-hands-ai/openhands:0.20

