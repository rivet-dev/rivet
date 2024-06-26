#!/bin/sh
set -euf

export KUBECONFIG="$(bolt output project-root)/gen/k8s/kubeconfig/$(bolt output namespace).yml"

echo
echo "Token:"
kubectl -n kubernetes-dashboard create token admin-user

echo
echo "Url:"
echo "http://localhost:8001/api/v1/namespaces/kubernetes-dashboard/services/https:kubernetes-dashboard:https/proxy/#/pod?namespace=_all"

kubectl proxy