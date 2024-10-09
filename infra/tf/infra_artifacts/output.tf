output "job_runner_binary_key" {
	value = aws_s3_object.job_runner_binary_upload.key
}

output "container_runner_binary_key" {
	value = aws_s3_object.container_runner_binary_upload.key
}

output "pegboard_manager_binary_key" {
	value = aws_s3_object.pegboard_manager_binary_upload.key
}
