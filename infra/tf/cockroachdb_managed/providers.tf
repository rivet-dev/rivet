provider "cockroach" {
	apikey = module.secrets.values["cockroachdb/api_key"]
}
