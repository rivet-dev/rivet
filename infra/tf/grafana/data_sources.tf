locals {
	client_id = module.secrets.values["cloudflare/access/grafana_cloud/client_id"]
	client_secret = module.secrets.values["cloudflare/access/grafana_cloud/client_secret"]
}

resource "grafana_data_source" "prometheus_svc" {
	type = "prometheus"
	name = "${var.namespace}-prometheus-svc"
	url = "https://prometheus-svc.${var.domain_main}"

	http_headers = {
		"CF-Access-Client-Id" = local.client_id
		"CF-Access-Client-Secret" = local.client_secret
	}

	json_data_encoded = jsonencode({
		httpMethod = "POST"
		prometheusType = "Prometheus"
		prometheusVersion = "2.40.0"
	})
}

resource "grafana_data_source" "prometheus_job" {
	type = "prometheus"
	name = "${var.namespace}-prometheus-job"
	url = "https://prometheus-job.${var.domain_main}"

	http_headers = {
		"CF-Access-Client-Id" = local.client_id
		"CF-Access-Client-Secret" = local.client_secret
	}

	json_data_encoded = jsonencode({
		httpMethod = "POST"
		prometheusType = "Prometheus"
		prometheusVersion = "2.40.0"
	})
}

resource "grafana_data_source" "clickhoue" {
	type = "grafana-clickhouse-datasource"
	name = "${var.namespace}-clickhouse"

	json_data_encoded = jsonencode({
		port = 443
		protocol = "http"
		secure = true
		server = "clickhouse-http.${var.domain_main}"
		username = "grafana"
	})

	secure_json_data_encoded = jsonencode({
		password = module.secrets.values["clickhouse/users/grafana/password"]
	})
}

