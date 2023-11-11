variable "namespace" {
	type = string
}

variable "clickhouse_tier" {
	type = string
}

variable "clickhouse_min_total_memory_gb" {
	type = number
	nullable = true
	default = null
}

variable "clickhouse_max_total_memory_gb" {
	type = number
	nullable = true
	default = null
}

