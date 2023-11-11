#!/bin/sh
set -euf

k3d cluster delete "rivet-$(bolt output namespace)"

(cd infra/tf/k8s_cluster_k3d/ && terraform state rm k3d_cluster.main)
