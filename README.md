# region-proxy

[![CI](https://github.com/M-Igashi/region-proxy/actions/workflows/ci.yml/badge.svg)](https://github.com/M-Igashi/region-proxy/actions/workflows/ci.yml)
[![GitHub Downloads](https://img.shields.io/github/downloads/M-Igashi/region-proxy/total?label=Downloads)](https://github.com/M-Igashi/region-proxy/releases)
[![GitHub Release](https://img.shields.io/github/v/release/M-Igashi/region-proxy)](https://github.com/M-Igashi/region-proxy/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-blue.svg)](https://www.rust-lang.org/)

A CLI tool to create a SOCKS proxy through AWS EC2 in any region. Useful when you need to access region-restricted content or services from a specific geographic location.

## Features

- 🌍 **Multi-region support**: Launch a proxy in any AWS region
- 🚀 **One command setup**: Automatically handles EC2 instance, security groups, and SSH keys
- ⚙️ **Configurable defaults**: Set default region, port, and other preferences
- 🔒 **Secure**: Uses SSH dynamic port forwarding (SOCKS5 proxy)
- 🍎 **macOS integration**: Automatically configures system-wide SOCKS proxy
- 🧹 **Clean shutdown**: Automatically terminates EC2 instance and cleans up all AWS resources
- 💰 **Cost-effective**: Uses the smallest instance types (t4g.nano/t3.nano)

## Prerequisites

- macOS (Linux support coming soon)
- AWS account with appropriate IAM permissions (see [AWS Setup](#aws-setup))
- AWS CLI ($ brew install awscli)
- Rust 1.75+ (only for building from source)

## Installation

### From source

```bash
git clone https://github.com/M-Igashi/region-proxy.git
cd region-proxy
cargo install --path .
```

### Homebrew

```bash
brew tap M-Igashi/tap
brew install region-proxy
```

## AWS Setup

### 1. Create IAM Policy

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

**Steps to create the policy:**

1. Go to [AWS IAM Console](https://console.aws.amazon.com/iam/) → **Policies** → **Create policy**
2. Select **JSON** tab and paste the above policy
3. Click **Next**, enter policy name (e.g., `region-proxy-ec2-policy`)
4. Click **Create policy**

### 2. Create IAM User and Attach Policy

1. Go to **Users** → **Create user**
2. Enter username and click **Next**
3. Select **Attach policies directly**
4. Search for and select your created policy (`region-proxy-ec2-policy`)
5. Click **Next** → **Create user**

### 3. Create Access Key

1. Go to **Users** → Select your user → **Security credentials**
2. Click **Create access key**
3. Select **Command Line Interface (CLI)**
4. Check the confirmation checkbox and click **Next**
5. Click **Create access key**
6. **Important**: Copy both **Access key ID** and **Secret access key** (shown only once!)

### 4. Configure AWS CLI

```bash
aws configure
```

Enter the following when prompted:
- **AWS Access Key ID**: Your access key (starts with `AKIA...`)
- **AWS Secret Access Key**: Your secret key
- **Default region name**: `ap-northeast-1` (or your preferred region)
- **Default output format**: `json`

## Usage

### Quick Start

Set your default region once:

```bash
region-proxy config set-region ap-northeast-1
```

Then start with a single command:

```bash
region-proxy start
```

### Start a proxy

```bash
# Start using default region (must be configured first)
region-proxy start

# Start a proxy in a specific region
region-proxy start --region ap-northeast-1

# Start a proxy with custom port
region-proxy start --port 8080

# Start without configuring system proxy
region-proxy start --no-system-proxy
```

### Check status

```bash
region-proxy status
```

### Stop the proxy

```bash
region-proxy stop

# Force stop (continues even if some cleanup operations fail)
region-proxy stop --force
```

### List available regions

```bash
region-proxy list-regions

# With details
region-proxy list-regions --detailed
```

### Configuration

Configure default settings that persist across sessions:

```bash
# Show current configuration
region-proxy config show

# Set default region
region-proxy config set-region ap-northeast-1

# Set default port
region-proxy config set-port 8080

# Set default instance type
region-proxy config set-instance-type t4g.micro

# Skip system proxy configuration by default
region-proxy config set-no-system-proxy true

# Clear a specific setting
region-proxy config unset region

# Reset all configuration
region-proxy config reset
```

Configuration is stored in `~/.region-proxy/config.json`.

### Cleanup orphaned resources

If the tool crashes or is interrupted, resources might be left behind. Use cleanup to remove them:

```bash
# Cleanup all regions
region-proxy cleanup

# Cleanup specific region
region-proxy cleanup --region ap-northeast-1
```

## How it works

1. **EC2 Launch**: Creates a minimal EC2 instance (t4g.nano for ARM-supported regions) with Amazon Linux 2023
2. **Security Group**: Creates a temporary security group allowing SSH access
3. **Key Pair**: Generates a temporary SSH key pair
4. **SSH Tunnel**: Establishes an SSH connection with dynamic port forwarding (-D option)
5. **System Proxy**: Configures macOS to use the SOCKS proxy (optional)

When you stop the proxy:
1. Disables the system SOCKS proxy
2. Terminates the SSH connection
3. Terminates the EC2 instance
4. Deletes the security group and key pair
5. Removes local key files

## Cost

The tool uses the smallest available instance types:
- **t4g.nano** (ARM regions): ~$0.0042/hour (~$3/month if running 24/7)
- **t3.nano** (x86 regions): ~$0.0052/hour (~$4/month if running 24/7)

**Note**: You're only charged while the instance is running. Remember to stop the proxy when not in use!

## Configuration Files

The tool stores its configuration and state in `~/.region-proxy/`:

```
~/.region-proxy/
├── config.json   # User preferences (default region, port, etc.)
├── state.json    # Current proxy state
└── keys/         # Temporary SSH keys
```

## Troubleshooting

### "AuthFailure" or "AWS was not able to validate the provided access credentials"

Your AWS credentials are invalid or not configured properly:

1. Verify your credentials are set:
   ```bash
   cat ~/.aws/credentials
   ```

2. Ensure the Access Key ID starts with `AKIA...`

3. Reconfigure if needed:
   ```bash
   aws configure
   ```

4. Check that your IAM user has the required permissions (see [AWS Setup](#aws-setup))

### "No AWS credentials found"

Make sure you have AWS credentials configured:

```bash
# Option 1: AWS CLI configuration
aws configure

# Option 2: Environment variables
export AWS_ACCESS_KEY_ID=your_access_key
export AWS_SECRET_ACCESS_KEY=your_secret_key
```

### "No region specified"

Either specify a region with `--region` or set a default:

```bash
region-proxy config set-region ap-northeast-1
```

### "Permission denied" when starting SSH tunnel

The tool automatically sets the correct permissions on the SSH key file. If you still encounter issues, check that the EC2 instance has started correctly:

```bash
region-proxy status
```

### "UnauthorizedOperation" errors

Your IAM user lacks the required EC2 permissions. Make sure you've attached the correct IAM policy (see [AWS Setup](#aws-setup)).

### Orphaned resources

If you see unexpected AWS resources with "region-proxy" tags:

```bash
region-proxy cleanup
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT License - see [LICENSE](LICENSE) for details.
