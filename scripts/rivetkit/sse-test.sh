#!/bin/sh

curl -X POST -d '{ "query": { "getOrCreateForTags": { "tags": { "name": "counter" }, "create": { "tags": { "name": "counter" } } } } }' http://localhost:8080/manager/actors

