# Wait for all Helm releases with daemon sets to finish deploying
#
# If we deploy the daemon sets after deployments, the deploy will fail
# since there will not be space on the nodes for the daemons
#
# If this happens, the pods need to be manually deleted so the daemons
# can schedule
resource "null_resource" "daemons" {
	depends_on = [helm_release.promtail, helm_release.pvc_exporter, helm_release.prometheus, helm_release.loki]
}

