# region-proxy

[![CI](https://github.com/M-Igashi/region-proxy/actions/workflows/ci.yml/badge.svg)](https://github.com/M-Igashi/region-proxy/actions/workflows/ci.yml)
[![GitHub Downloads](https://img.shields.io/github/downloads/M-Igashi/region-proxy/total?label=Downloads)](https://github.com/M-Igashi/region-proxy/releases)
[![GitHub Release](https://img.shields.io/github/v/release/M-Igashi/region-proxy)](https://github.com/M-Igashi/region-proxy/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A CLI tool to create a SOCKS proxy through AWS EC2 in any region.

<p align="center">
  <img src="docs/demo.gif" alt="region-proxy demo" width="800">
</p>

## Quick Start

```bash
# 1. Install
brew tap M-Igashi/tap
brew install region-proxy

# 2. Set your default region (one-time setup)
region-proxy config set-region ap-northeast-1

# 3. Start the proxy
region-proxy start

# 4. Verify your IP is now in Tokyo
curl ipinfo.io

# 5. Stop when done
region-proxy stop
```

That's it! After the initial setup, just run `region-proxy start` and `region-proxy stop`.

## Features

- **Multi-region support** – Launch a proxy in any of 17 AWS regions
- **One command setup** – Automatically handles EC2 instance, security groups, and SSH keys
- **Configurable defaults** – Set default region, port, and other preferences
- **Secure** – Uses SSH dynamic port forwarding (SOCKS5 proxy)
- **macOS integration** – Automatically configures system-wide SOCKS proxy
- **Clean shutdown** – Automatically terminates EC2 instance and cleans up all AWS resources
- **Cost-effective** – Uses the smallest instance types (t4g.nano/t3.nano, ~$0.004/hour)

## Prerequisites

- macOS
- AWS account with EC2 permissions
- AWS CLI configured (`aws configure`)

## Installation

### Homebrew (Recommended)

```bash
brew tap M-Igashi/tap
brew install region-proxy
```

### From Source

```bash
git clone https://github.com/M-Igashi/region-proxy.git
cd region-proxy
cargo install --path .
```

## Usage

### Start a proxy

```bash
region-proxy start                              # Use default region
region-proxy start --region us-west-2           # Specify region
region-proxy start --port 8080                  # Custom port
region-proxy start --no-system-proxy            # Don't configure system proxy
```

### Check status

```bash
region-proxy status
```

### Stop the proxy

```bash
region-proxy stop
region-proxy stop --force    # Continue even if some cleanup fails
```

### List available regions

```bash
region-proxy list-regions
region-proxy list-regions --detailed
```

### Configuration

```bash
region-proxy config show                        # Show current config
region-proxy config set-region ap-northeast-1   # Set default region
region-proxy config set-port 8080               # Set default port
region-proxy config reset                       # Reset all settings
```

### Cleanup orphaned resources

```bash
region-proxy cleanup
region-proxy cleanup --region ap-northeast-1
```

## AWS Setup

### Option 1: Terraform (Recommended)

```bash
cd terraform
terraform init
terraform apply

# Configure AWS CLI with the created credentials
aws configure
# Enter the access_key_id and secret_access_key from terraform output
terraform output -raw secret_access_key
```

### Option 2: CloudFormation

```bash
aws cloudformation create-stack \
  --stack-name region-proxy-iam \
  --template-body file://cloudformation/region-proxy-iam.yaml \
  --capabilities CAPABILITY_NAMED_IAM

# Wait for stack creation
aws cloudformation wait stack-create-complete --stack-name region-proxy-iam

# Get credentials
aws cloudformation describe-stacks --stack-name region-proxy-iam \
  --query 'Stacks[0].Outputs'

# Configure AWS CLI with the credentials
aws configure
```

### Option 3: Manual Setup

Create an IAM policy with the following permissions:

```json
{
    "Version": "2012-10-17",
    "Statement": [
        {
            "Effect": "Allow",
            "Action": [
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
            ],
            "Resource": "*"
        }
    ]
}
```

Then configure AWS CLI:

```bash
aws configure
```

## How it works

1. Creates a minimal EC2 instance (t4g.nano) with Amazon Linux 2023
2. Creates a temporary security group allowing SSH access
3. Generates a temporary SSH key pair
4. Establishes an SSH tunnel with dynamic port forwarding
5. Optionally configures macOS system proxy

When you stop the proxy, all resources are automatically cleaned up.

## Troubleshooting

**AuthFailure or credential errors**

```bash
aws sts get-caller-identity
```

**No region specified**

```bash
region-proxy config set-region ap-northeast-1
```

**Orphaned resources**

```bash
region-proxy cleanup
```

## License

MIT License - see [LICENSE](LICENSE) for details.
