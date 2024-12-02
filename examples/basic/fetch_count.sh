#!/bin/sh

# curl -X POST -d '{"key":"test"}' 'http://localhost:6420/modules/actors_test/scripts/fetch_counter/call'
curl -X POST -d '{"key":"test"}' 'https://test-game-cir.backend.rivet.gg/modules/actors_test/scripts/fetch_counter/call' | jq

