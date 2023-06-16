locals {
	master_conf = file("${path.module}/files/master.conf")
	salt_version = "3006.1"
}
