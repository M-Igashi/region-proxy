# region-proxy

[![CI](https://github.com/M-Igashi/region-proxy/actions/workflows/ci.yml/badge.svg)](https://github.com/M-Igashi/region-proxy/actions/workflows/ci.yml)
[![GitHub Downloads](https://img.shields.io/github/downloads/M-Igashi/region-proxy/total?label=Downloads)](https://github.com/M-Igashi/region-proxy/releases)
[![GitHub Release](https://img.shields.io/github/v/release/M-Igashi/region-proxy)](https://github.com/M-Igashi/region-proxy/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-blue.svg)](https://www.rust-lang.org/)

A CLI tool to create a SOCKS proxy through AWS EC2 in any region. Useful when you need to access region-restricted content or services from a specific geographic location.

## Demo

![region-proxy demo](docs/demo.gif)

## Features

- 🌍 **Multi-region support**: Launch a proxy in any of 33 AWS regions
- 🚀 **One command setup**: Automatically handles EC2 instance, security groups, and SSH keys
- ⚙️ **Configurable defaults**: Set default region, port, and other preferences
- 🔒 **Secure**: Uses SSH dynamic port forwarding (SOCKS5 proxy)
- 🍎 **macOS integration**: Automatically configures system-wide SOCKS proxy
- 🧹 **Clean shutdown**: Automatically terminates EC2 instance and cleans up all AWS resources
- 💰 **Cost-effective**: Uses the smallest instance types (t4g.nano/t3.nano)

## Use Cases

- 🎮 **Gaming**: Access region-locked game servers or content
- 📺 **Streaming**: Watch region-restricted video content
- 🧪 **Testing**: Test your application from different geographic locations
- 🔒 **Privacy**: Route traffic through a different region
- 💼 **Development**: Access region-specific APIs or services

## Why region-proxy?

| Feature | region-proxy | Manual EC2 | VPN Services |
|---------|-------------|------------|--------------|
| Setup time | ~30 seconds | ~10 minutes | Varies |
| Cost | ~$0.004/hour | Same | $5-15/month |
| AWS regions | All 33 | All 33 | Limited |
| Auto cleanup | ✅ | ❌ | N/A |
| No subscription | ✅ | ✅ | ❌ |
| Open source | ✅ | N/A | ❌ |

## Quick Start

```bash
# Install
brew tap M-Igashi/tap
brew install region-proxy

# Start proxy in Tokyo
region-proxy start --region ap-northeast-1

# Your traffic now routes through Tokyo!
curl ipinfo.io  # Shows Tokyo IP

# Stop and cleanup
region-proxy stop
```

## Architecture

```
┌─────────────┐     SSH Tunnel      ┌──────────────┐     ┌──────────────┐
│   Your Mac  │ ──────────────────► │  EC2 (Tokyo) │ ──► │   Internet   │
│  SOCKS:1080 │    Port Forwarding  │   t4g.nano   │     │  (JP region) │
└─────────────┘                     └──────────────┘     └──────────────┘
```

## Prerequisites

- macOS (Linux support coming soon)
- AWS account with appropriate IAM permissions (see [AWS Setup](#aws-setup))
- AWS CLI configured (`brew install awscli && aws configure`)

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
# Start using default region (configure first with config command)
region-proxy start

# Start in a specific region
region-proxy start --region ap-northeast-1

# Start with custom port
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

# With instance type details
region-proxy list-regions --detailed
```

### Configuration

```bash
# Show current configuration
region-proxy config show

# Set default region
region-proxy config set-region ap-northeast-1

# Set default port
region-proxy config set-port 8080

# Reset all configuration
region-proxy config reset
```

### Cleanup orphaned resources

If the tool crashes or is interrupted, use cleanup to remove orphaned AWS resources:

```bash
region-proxy cleanup
region-proxy cleanup --region ap-northeast-1
```

## How it works

1. **EC2 Launch**: Creates a minimal EC2 instance (t4g.nano for ARM-supported regions) with Amazon Linux 2023
2. **Security Group**: Creates a temporary security group allowing SSH access only from your IP
3. **Key Pair**: Generates a temporary SSH key pair
4. **SSH Tunnel**: Establishes an SSH connection with dynamic port forwarding (-D option)
5. **System Proxy**: Configures macOS to use the SOCKS proxy (optional)

When you stop the proxy, all resources are automatically cleaned up.

## Cost

| Instance | Hourly | Daily (8hr) | Monthly (24/7) |
|----------|--------|-------------|----------------|
| t4g.nano | $0.0042 | $0.034 | ~$3 |
| t3.nano | $0.0052 | $0.042 | ~$4 |

**You're only charged while the instance is running.** Remember to stop the proxy when not in use!

## Security

- 🔑 SSH keys are generated per-session and automatically deleted
- 🛡️ Security groups allow only your IP address
- 💾 EC2 instances are terminated on stop (no persistent data)
- 🏠 All credentials stay local (never transmitted except to AWS)

## AWS Setup

### 1. Create IAM Policy

Create an IAM policy with EC2 permissions:

<details>
<summary>Click to expand IAM policy JSON</summary>

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

</details>

### 2. Create IAM User and Attach Policy

1. Go to [AWS IAM Console](https://console.aws.amazon.com/iam/) → **Users** → **Create user**
2. Attach the policy created above
3. Create access key (CLI use case)

### 3. Configure AWS CLI

```bash
aws configure
# Enter your Access Key ID, Secret Access Key, and preferred region
```

## Troubleshooting

<details>
<summary>AuthFailure or credential errors</summary>

Your AWS credentials are invalid. Verify with:
```bash
cat ~/.aws/credentials
aws sts get-caller-identity
```
</details>

<details>
<summary>No region specified</summary>

Set a default region:
```bash
region-proxy config set-region ap-northeast-1
```
</details>

<details>
<summary>UnauthorizedOperation errors</summary>

Your IAM user lacks required permissions. Ensure the IAM policy is correctly attached.
</details>

<details>
<summary>Orphaned resources</summary>

```bash
region-proxy cleanup
```
</details>

## Alternatives

- **AWS SSM Session Manager**: More secure (no SSH ports), but more complex setup
- **Commercial VPNs**: More regions, mobile support, but subscription-based
- **Manual EC2 + SSH**: Full control, but tedious setup and cleanup

## Roadmap

- [ ] Linux support
- [ ] Multiple simultaneous connections
- [ ] Connection time limits
- [ ] Cost estimation before start
- [ ] IPv6 support

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT License - see [LICENSE](LICENSE) for details.

---

<p align="center">
  <a href="https://github.com/M-Igashi/region-proxy">⭐ Star this repo if you find it useful!</a>
</p>
