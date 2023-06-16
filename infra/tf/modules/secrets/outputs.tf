output "values" {
	value = {
		for key in var.keys:
		key => data.external.bolt_secret[key].result["value"]
	}
}

