variable "namespace" {
	type = string
}

variable "redis_dbs" {
	type = map(object({
	}))
}
