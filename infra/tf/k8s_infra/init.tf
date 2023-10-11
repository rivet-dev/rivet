resource "kubernetes_namespace" "rivet_service" {
	metadata {
		name = "rivet-service"
	}
}

# Used by shells and migrations
resource "kubernetes_namespace" "bolt" {
	metadata {
		name = "bolt"
	}
}

# TODO: These configmaps mounted to pods don't update the pods when changed
resource "kubernetes_config_map" "health_checks" {
	metadata {
		name = "health-checks"
		namespace = kubernetes_namespace.rivet_service.metadata.0.name
	}

	data = {
		"health-checks.sh" = <<-EOF
			#!/bin/sh
			set -uf

			# Log to file
			exec >> "/var/log/health-checks.txt" 2>&1

			# Install curl
			apt-get update -y
			apt-get install -y curl

			curl 127.0.0.1:${var.k8s_health_port}/health/liveness
			EXIT_STATUS=$?
			if [ $EXIT_STATUS -ne 0 ]; then
				echo "health server liveness check failed"
				exit $EXIT_STATUS
			fi

			curl 127.0.0.1:${var.k8s_health_port}/health/crdb/db-user
			EXIT_STATUS=$?
			if [ $EXIT_STATUS -ne 0 ]; then
				echo "cockroach check failed"
				exit $EXIT_STATUS
			fi

			curl 127.0.0.1:${var.k8s_health_port}/health/nats
			EXIT_STATUS=$?
			if [ $EXIT_STATUS -ne 0 ]; then
				echo "nats connection check failed"
				exit $EXIT_STATUS
			fi

			curl 127.0.0.1:${var.k8s_health_port}/health/redis/redis-chirp
			EXIT_STATUS=$?
			if [ $EXIT_STATUS -ne 0 ]; then
				echo "redis chirp connection check failed"
				exit $EXIT_STATUS
			fi

			# Static endpoint flag
			if [[ "$*" == *"--static"* ]]; then
				curl 127.0.0.1:${var.k8s_health_port}/
				EXIT_STATUS=$?
				if [ $EXIT_STATUS -ne 0 ]; then
					echo "static root accessible check failed"
					exit $EXIT_STATUS
				fi
			fi

			echo Ok
			echo
			EOF
	}
}

resource "kubernetes_config_map" "install_ca" {
	metadata {
		name = "install-ca"
		namespace = kubernetes_namespace.rivet_service.metadata.0.name
	}

	data = {
		"install-ca.sh" = <<-EOF
			set -euf

			# Log to file
			exec >> "/var/log/install-ca.txt" 2>&1

			# Merge CA certificates provided from other config maps for self-signed TLS connections to databases
			#
			# Overriding LD_LIBRARY_PATH prevents apt from using the OpenSSL installation from /nix/store (if mounted).
			LD_LIBRARY_PATH=/lib:/usr/lib:/usr/local/lib update-ca-certificates
			EOF
	}
}

module "docker_auth" {
	source = "../modules/k8s_auth"

	namespaces = [
		for x in [
			kubernetes_namespace.traffic_server,
			# kubernetes_namespace.redis_exporter,
			kubernetes_namespace.rivet_service,
			kubernetes_namespace.imagor,
			kubernetes_namespace.nsfw_api
		]:
		x.metadata.0.name
	]
	authenticate_all_docker_hub_pulls = var.authenticate_all_docker_hub_pulls
	deploy_method_cluster = var.deploy_method_cluster
}
