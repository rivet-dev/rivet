locals {
	container_runner_src_dir = "${path.module}/../../../lib/pegboard/container-runner"

	container_runner_src_files = flatten([
		["${local.container_runner_src_dir}/Cargo.toml"],
		[for f in fileset("${local.container_runner_src_dir}/src/", "**/*"): "${local.container_runner_src_dir}/src/${f}"],
	])
	container_runner_src_hash = md5(join("", [
		for f in local.container_runner_src_files: filemd5(f)
	]))
	container_runner_dst_binary_path = "/tmp/container-runner-${local.container_runner_src_hash}"
}

resource "null_resource" "container_runner_build" {
	triggers = {
		container_runner_src_hash = local.container_runner_src_hash
		container_runner_dst_binary_path = local.container_runner_dst_binary_path
	}

	provisioner "local-exec" {
		command = <<-EOT
		#!/bin/bash
		set -euf

		# Variables
		IMAGE_NAME="container-runner:${local.container_runner_src_hash}"
		CONTAINER_NAME="temp-container-runner-${local.container_runner_src_hash}"
		BINARY_PATH_IN_CONTAINER="/app/target/x86_64-unknown-linux-musl/release/container-runner"
		DST_BINARY_PATH="${local.container_runner_dst_binary_path}"

		# Build the Docker image
		docker build --platform linux/amd64 -t $IMAGE_NAME '${local.container_runner_src_dir}'

		# Create a temporary container
		docker create --platform linux/amd64 --name $CONTAINER_NAME $IMAGE_NAME

		# Copy the binary from the container to the host
		docker cp $CONTAINER_NAME:$BINARY_PATH_IN_CONTAINER $DST_BINARY_PATH

		# Remove the temporary container
		docker rm $CONTAINER_NAME
		EOT
	}
}

resource "aws_s3_object" "container_runner_binary_upload" {
	depends_on = [null_resource.container_runner_build]

	lifecycle {
		prevent_destroy = true
	}

	bucket = local.artifacts_bucket_name
	key = "container-runner/${local.container_runner_src_hash}/container-runner"
	source = local.container_runner_dst_binary_path
}

