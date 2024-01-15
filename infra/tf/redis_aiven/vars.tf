variable "namespace" {
	type = string
}

variable "redis_dbs" {
	type = map(object({
		persistent =  bool
	}))
}

variable "redis_aiven" {
	type = object({
		project = string
		cloud = string
		plan_ephemeral = string
		plan_persistent = string
	})
}

