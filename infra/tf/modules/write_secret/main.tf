resource "null_resource" "example" {
	triggers = var.triggers

	provisioner "local-exec" {
		# HACK: jsonencode is an imperfect encoding of strings to safe encoding in bash
		command = "bolt secret set ${jsonencode(var.path)} ${jsonencode(var.value)}"
	}
}


