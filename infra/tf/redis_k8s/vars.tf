variable "redis_dbs" {
	type = map(object({
		persistent =  bool
	}))
}

