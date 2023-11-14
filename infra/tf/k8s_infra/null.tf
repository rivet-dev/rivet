# Wait for all Helm releases with Daemon sets to finish deploying
resource "null_resource" "daemons" {
	depends_on = [helm_release.promtail, helm_release.pvc_exporter, helm_release.prometheus, helm_release.loki]
}

