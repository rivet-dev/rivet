provider "grafana" {
	url  = "https://rivetgg.grafana.net/"
	auth = module.secrets.values["grafana/terraform/token"]
}

