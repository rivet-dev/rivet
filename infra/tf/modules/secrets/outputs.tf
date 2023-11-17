# IMPORTANT: When using optional = true, the returned value is an EMPTY STRING not `null`
output "values" {
	value = {
		for key in var.keys:
		key => data.external.bolt_secret[key].result["value"]
	}
}
