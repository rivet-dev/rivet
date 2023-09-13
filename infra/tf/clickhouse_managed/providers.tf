provider "clickhouse" {
	organization_id = module.secrets.values["clickhouse_cloud/organization_id"]
	token_key = module.secrets.values["clickhouse_cloud/token_key"]
	token_secret = module.secrets.values["clickhouse_cloud/token_secret"]
}

