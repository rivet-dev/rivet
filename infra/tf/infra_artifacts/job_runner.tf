locals {
	job_runner_src_dir = "${path.module}/../../../lib/job-runner"

	job_runner_src_files = flatten([
		["${local.job_runner_src_dir}/Cargo.toml"],
		[for f in fileset("${local.job_runner_src_dir}/src/", "**/*"): "${local.job_runner_src_dir}/src/${f}"],
	])
	job_runner_src_hash = md5(join("", [
		for f in local.job_runner_src_files: filemd5(f)
	]))
	job_runner_dst_binary_path = "/tmp/job-runner-${local.job_runner_src_hash}"
}

resource "null_resource" "job_runner_build" {
	triggers = {
		job_runner_src_hash = local.job_runner_src_hash
		job_runner_dst_binary_path = local.job_runner_dst_binary_path
	}

	provisioner "local-exec" {
		command = <<-EOT
		#!/bin/bash
		set -euf

		# Variables
		IMAGE_NAME="job-runner:${local.job_runner_src_hash}"
		CONTAINER_NAME="temp-job-runner-${local.job_runner_src_hash}"
		BINARY_PATH_IN_CONTAINER="/app/target/x86_64-unknown-linux-musl/release/job-runner"
		DST_BINARY_PATH="${local.job_runner_dst_binary_path}"

		# Build the Docker image
		docker build --platform linux/amd64 -t $IMAGE_NAME '${local.job_runner_src_dir}'

		# Create a temporary container
		docker create --name $CONTAINER_NAME $IMAGE_NAME

		# Copy the binary from the container to the host
		docker cp $CONTAINER_NAME:$BINARY_PATH_IN_CONTAINER $DST_BINARY_PATH

		# Remove the temporary container
		docker rm $CONTAINER_NAME
		EOT
	}
}

resource "aws_s3_object" "job_runner_binary_upload" {
	depends_on = [null_resource.job_runner_build]

	lifecycle {
		prevent_destroy = true
	}

	bucket = "${var.namespace}-bucket-infra-artifacts"
	key = "job-runner/${local.job_runner_src_hash}/job-runner"
	source = local.job_runner_dst_binary_path
}

