# In order to cleanly grant permission to access the cluster using AWS policies, we need to create a:
# - Role for the user to assume
# - Policy to attach to a user or group to allow them to assume the role
#
# Example: https://eng.grip.security/enabling-aws-iam-group-access-to-an-eks-cluster-using-rbac#heading-step-by-step-guide
#
# You need to manually attach aws_iam_role.eks_admin_assume_role to the IAM user/group you want to grant access to the cluster.

locals {
	eks_admin_username = "aws-admin"
}

# Create a role used to authenticate with the cluster
resource "aws_iam_role" "eks_admin" {
	name = "${local.name}-Admin"

	assume_role_policy = jsonencode({
		Version = "2012-10-17",
		Statement = [
			{
				Effect = "Allow",
				Principal = {
					# Allow access to all users in AWS account
					AWS = "arn:${local.partition}:iam::${local.account_id}:root"
				},
				Action = "sts:AssumeRole",
			}
		]
	})
}

# Create policy that can be attached to a user/group to allow them to assume the k8s admin role
resource "aws_iam_policy" "eks_admin_assume_role" {
	name = "${local.name}-EKSAdminAssumeRole"

	policy = jsonencode({
		Version = "2012-10-17"
		Statement = [
			{
				Effect = "Allow"
				Action = "sts:AssumeRole"
				Resource = aws_iam_role.eks_admin.arn
			}
		]
	})
}

output "eks_admin_role_arn" {
	value = aws_iam_role.eks_admin.arn
}

