locals {
	db_configs = {
		ephemeral = {
			plan = var.redis_aiven.plan_ephemeral
			maxmemory_policy = "allkeys-lru"
			persistence = "off"
		}
		persistent = {
			plan = var.redis_aiven.plan_persistent
			maxmemory_policy = "noeviction"
			persistence = "rdb"
		}
	}
}

module "secrets" {
	source = "../modules/secrets"

	keys = flatten([
		["aiven/api_token"],
		[
			for k, v in var.redis_dbs:
			[
				"redis/${k}/username",
				"redis/${k}/password",
			]
		],
	])
}

resource "aiven_redis" "main" {
	for_each = var.redis_dbs

	project = var.redis_aiven.project
	cloud_name = var.redis_aiven.cloud
	plan = local.db_configs[each.key].plan
	service_name = "rivet-${var.namespace}-${each.key}"
	maintenance_window_dow = "monday"
	maintenance_window_time = "10:00:00"

	tag {
		key = "rivet:namespace"
		value = var.namespace
	}

	redis_user_config {
		redis_ssl = true
		redis_maxmemory_policy = local.db_configs[each.key].maxmemory_policy
		redis_persistence = local.db_configs[each.key].persistence

		dynamic "ip_filter_object" {
			for_each = sort(data.terraform_remote_state.k8s_cluster_aws.outputs.nat_public_ips)

			content {
				network = "${ip_filter_object.value}/32"
				description = "AWS NAT"
			}
		}

		public_access {
			redis = true
		}
	}
}

resource "aiven_redis_user" "main" {
	for_each = var.redis_dbs

	project = var.redis_aiven.project
	service_name = aiven_redis.main[each.key].service_name
	username = module.secrets.values["redis/${each.key}/username"]
	password = module.secrets.values["redis/${each.key}/password"]

	redis_acl_categories = ["+@all"]
	redis_acl_commands = []
	redis_acl_channels = ["*"]
	redis_acl_keys = ["*"]
}

