locals {
	pegboard_manager_src_dir = "${path.module}/../../../lib/pegboard-manager"

	pegboard_manager_src_files = flatten([
		["${local.pegboard_manager_src_dir}/Cargo.toml", "${local.pegboard_manager_src_dir}/Cargo.lock"],
		[for f in fileset("${local.pegboard_manager_src_dir}/src/", "**/*"): "${local.pegboard_manager_src_dir}/src/${f}"],
	])
	pegboard_manager_src_hash = md5(join("", [
		for f in local.pegboard_manager_src_files: filemd5(f)
	]))
	pegboard_manager_dst_binary_path = "/tmp/pegboard-manager-${local.pegboard_manager_src_hash}"
}

resource "null_resource" "pegboard_manager_build" {
	triggers = {
		pegboard_manager_src_hash = local.pegboard_manager_src_hash
		pegboard_manager_dst_binary_path = local.pegboard_manager_dst_binary_path
	}

	provisioner "local-exec" {
		command = <<-EOT
		#!/bin/bash
		set -euf

		# Variables
		IMAGE_NAME="pegboard-manager:${local.pegboard_manager_src_hash}"
		CONTAINER_NAME="temp-pegboard-manager-${local.pegboard_manager_src_hash}"
		BINARY_PATH_IN_CONTAINER="/app/target/x86_64-unknown-linux-musl/release/pegboard-manager"
		DST_BINARY_PATH="${local.pegboard_manager_dst_binary_path}"

		# Build the Docker image
		docker build --platform linux/amd64 -t $IMAGE_NAME '${local.pegboard_manager_src_dir}'

		# Create a temporary container
		docker create --name $CONTAINER_NAME $IMAGE_NAME

		# Copy the binary from the container to the host
		docker cp $CONTAINER_NAME:$BINARY_PATH_IN_CONTAINER $DST_BINARY_PATH

		# Remove the temporary container
		docker rm $CONTAINER_NAME
		EOT
	}
}

resource "aws_s3_object" "pegboard_manager_binary_upload" {
	depends_on = [null_resource.pegboard_manager_build]

	lifecycle {
		prevent_destroy = true
	}

	bucket = "${var.namespace}-bucket-infra-artifacts"
	key = "pegboard-manager/${local.pegboard_manager_src_hash}/pegboard-manager"
	source = local.pegboard_manager_dst_binary_path
}

