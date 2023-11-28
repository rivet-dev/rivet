#!/bin/sh
set -euf

export KUBECONFIG="$(bolt output project-root)/gen/k8s/kubeconfig/$(bolt output namespace).yml"
kubectl port-forward -n $FORWARD_NS $FORWARD_NAME ${PORT:-9090}:$FORWARD_PORT

