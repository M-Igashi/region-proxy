# Hacker News Post Draft

## Title (Choose one)

**Option A (Recommended):**
```
Show HN: Region-proxy – One-command SOCKS proxy through AWS EC2 in any region
```

**Option B:**
```
Show HN: CLI tool to bypass geo-restrictions using temporary EC2 instances (~$0.004/hr)
```

**Option C:**
```
Show HN: I built a Rust CLI that automates SSH tunneling through ephemeral EC2 instances
```

---

## Post Body

```
I built a CLI tool in Rust that creates SOCKS5 proxies through AWS EC2 instances in any of the 33 AWS regions.

Demo: https://raw.githubusercontent.com/M-Igashi/region-proxy/master/docs/demo.gif

The problem: I needed to access region-restricted content from specific countries, but commercial VPNs are expensive ($5-15/month) and don't cover all AWS regions.

The solution: region-proxy automates the entire workflow:

    $ region-proxy start --region ap-northeast-1
    🚀 Starting proxy in ap-northeast-1 (Tokyo)...
    ✅ Proxy ready at localhost:1080

    $ curl --socks5 localhost:1080 ipinfo.io
    {"city": "Tokyo", "country": "JP", ...}

    $ region-proxy stop
    ✅ All resources cleaned up

What it does:
- Launches a minimal EC2 instance (t4g.nano, ~$0.004/hr)
- Creates temporary security groups (your IP only) and SSH keys
- Sets up SSH dynamic port forwarding (-D flag)
- Configures macOS system proxy (optional)
- Cleans up everything on stop

Built with Rust, AWS SDK for Rust, tokio, and clap. Currently macOS only, Linux support coming.

GitHub: https://github.com/M-Igashi/region-proxy

Install: brew tap M-Igashi/tap && brew install region-proxy

Happy to answer questions about the implementation!
```

---

## Follow-up Comment (Post immediately after submission)

```
Author here. A few notes on the implementation:

1. Why Rust? Single binary distribution, easy cross-compilation for universal macOS binaries (ARM + x86), and the AWS SDK for Rust is surprisingly mature.

2. Cost breakdown: t4g.nano in most regions is $0.0042/hr. If you use it 2 hours/day, that's ~$0.25/month. Compare that to VPN subscriptions.

3. The trickiest part was cleanup reliability. If the tool crashes mid-operation, orphaned resources could cost money. Solution: state file + cleanup command:

    $ region-proxy cleanup --region ap-northeast-1

4. Security model:
   - SSH keys are generated per-session, deleted on stop
   - Security groups whitelist only your current IP
   - EC2 instances are terminated (not stopped), so no persistent data

5. Future plans: Linux support, multiple simultaneous proxies, connection time limits.

Source is MIT licensed. PRs welcome!
```

---

## Anticipated Q&A

### Q: Why not just use a commercial VPN?

Commercial VPNs work great for many use cases, but:
- Limited region coverage (most don't have all 33 AWS regions)
- Monthly subscriptions add up for occasional use
- You're trusting a third party with your traffic

With region-proxy, you control the infrastructure, pay per-use (~$0.004/hr), and can access any AWS region.

### Q: Isn't this just SSH tunneling with extra steps?

Yes! The value is in automation:
- Finding the latest Amazon Linux AMI for the region
- Creating/deleting security groups with proper rules
- Managing SSH keys
- Waiting for instance readiness
- Cleaning up to avoid forgotten charges

Without this, it's ~10 minutes of AWS console clicking per session.

### Q: Why not AWS SSM Session Manager?

SSM is more secure (no open SSH ports) but requires more setup:
- IAM role configuration
- SSM agent verification
- More complex IAM policies

region-proxy is simpler: just AWS credentials and one command.

### Q: Is this against AWS ToS?

No. You're launching EC2 instances for legitimate use (SSH proxy). This is a standard use case. Just don't abuse it for illegal activities.

### Q: Security concerns?

- SSH keys: generated per-session, deleted on stop
- Security groups: allow only your current IP
- EC2 instances: terminated (not stopped), no persistent data
- Credentials: stay local, only sent to AWS APIs

### Q: Why macOS only?

I built what I needed first. The core functionality works on Linux; it just needs the system proxy configuration part (`networksetup` on macOS → `gsettings` or env vars on Linux).

### Q: What about performance/latency?

It's an SSH tunnel, so expect some overhead. For browsing and API access, it's fine. For gaming or real-time applications, a proper VPN or direct connection would be better.

---

## Posting Tips

**Best times (Pacific Time):**
- Tuesday-Thursday, 8-10 AM PT
- Catches East Coast morning + Europe afternoon
- Avoid weekends and US holidays

**After posting:**
1. Post the follow-up comment immediately
2. Answer questions promptly (first 2 hours are critical)
3. Be humble, acknowledge limitations
4. Thank people for feedback

---

## Cross-posting Plan

After HN, consider posting to:

1. **Reddit** (wait 1-2 days after HN)
   - r/rust - "I built a CLI tool with AWS SDK for Rust"
   - r/aws - "Tool to create temporary SOCKS proxies via EC2"
   - r/commandline - "region-proxy: SOCKS proxy through AWS EC2"
   - r/selfhosted - Focus on self-hosting aspect

2. **Twitter/X**
   - Thread format with demo GIF
   - Tag #rustlang #aws #opensource #cli

3. **Dev.to** (longer form, can be similar to Zenn article)

4. **Lobsters** (if you have an invite)

---

## Success Metrics

Track before/after:
- [ ] GitHub stars
- [ ] Homebrew tap stars
- [ ] GitHub issues/PRs from new users  
- [ ] HN points and ranking
- [ ] Comments and engagement
