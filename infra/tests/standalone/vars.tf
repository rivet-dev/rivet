variable "namespace" {
	type = string
}

variable "private_key_openssh" {
	type = string
	sensitive = true
}

variable "linode_token" {
	type = string
	sensitive = true
}

variable "github_pat" {
	type = string
	sensitive = true
}

variable "repo_ref" {
	type = string
}

variable "namespace_config" {
	type = string
}

variable "namespace_secrets" {
	type = string
	sensitive = true
}

