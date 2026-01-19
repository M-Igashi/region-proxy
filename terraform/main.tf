terraform {
  required_version = ">= 1.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = ">= 4.0"
    }
  }
}

provider "aws" {}

variable "user_name" {
  description = "Name for the IAM user"
  type        = string
  default     = "region-proxy-user"
}

variable "create_access_key" {
  description = "Whether to create an access key for the user"
  type        = bool
  default     = true
}

# IAM Policy for region-proxy
resource "aws_iam_policy" "region_proxy" {
  name        = "region-proxy-policy"
  description = "Permissions required for region-proxy to manage EC2 instances"

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "ec2:DescribeImages",
          "ec2:DescribeInstances",
          "ec2:DescribeSecurityGroups",
          "ec2:DescribeKeyPairs",
          "ec2:RunInstances",
          "ec2:TerminateInstances",
          "ec2:CreateSecurityGroup",
          "ec2:DeleteSecurityGroup",
          "ec2:AuthorizeSecurityGroupIngress",
          "ec2:CreateKeyPair",
          "ec2:DeleteKeyPair",
          "ec2:CreateTags"
        ]
        Resource = "*"
      }
    ]
  })
}

# IAM User
resource "aws_iam_user" "region_proxy" {
  name = var.user_name
  tags = {
    Purpose = "region-proxy"
  }
}

# Attach policy to user
resource "aws_iam_user_policy_attachment" "region_proxy" {
  user       = aws_iam_user.region_proxy.name
  policy_arn = aws_iam_policy.region_proxy.arn
}

# Access key (optional)
resource "aws_iam_access_key" "region_proxy" {
  count = var.create_access_key ? 1 : 0
  user  = aws_iam_user.region_proxy.name
}

# Outputs
output "user_name" {
  description = "IAM user name"
  value       = aws_iam_user.region_proxy.name
}

output "user_arn" {
  description = "IAM user ARN"
  value       = aws_iam_user.region_proxy.arn
}

output "policy_arn" {
  description = "IAM policy ARN"
  value       = aws_iam_policy.region_proxy.arn
}

output "access_key_id" {
  description = "Access key ID (if created)"
  value       = var.create_access_key ? aws_iam_access_key.region_proxy[0].id : null
  sensitive   = true
}

output "secret_access_key" {
  description = "Secret access key (if created)"
  value       = var.create_access_key ? aws_iam_access_key.region_proxy[0].secret : null
  sensitive   = true
}
