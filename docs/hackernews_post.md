# Hacker News Post Draft

## Title Options (pick one)

**Option A (Direct):**
```
Show HN: Region-proxy – Create SOCKS proxies through AWS EC2 in any region
```

**Option B (Problem-focused):**
```
Show HN: CLI tool to bypass geo-restrictions using AWS EC2 (~$0.004/hr)
```

**Option C (Technical):**
```
Show HN: Rust CLI that automates SSH tunneling through temporary EC2 instances
```

---

## Post Body

### Short Version (Recommended for HN)

```
I built a CLI tool in Rust that creates SOCKS5 proxies through AWS EC2 instances in any of the 33 AWS regions.

The problem: I needed to access region-restricted content from specific countries, but commercial VPNs are expensive ($5-15/month) and don't cover all regions I needed.

The solution: region-proxy automates the entire workflow:

- Launches a minimal EC2 instance (t4g.nano, ~$0.004/hr)
- Creates temporary security groups and SSH keys
- Sets up SSH dynamic port forwarding
- Configures macOS system proxy (optional)
- Cleans up everything when you stop

Usage is simple:

    $ region-proxy start --region ap-northeast-1
    ✅ Proxy ready at localhost:1080

    $ region-proxy stop
    ✅ All resources cleaned up

Built with Rust, AWS SDK for Rust, tokio, and clap. Currently macOS only, Linux support coming soon.

GitHub: https://github.com/M-Igashi/region-proxy

Happy to answer any questions about the implementation or use cases.
```

---

## Anticipated Questions & Answers

### Q: Why not just use a commercial VPN?

A: Commercial VPNs are great for many use cases, but they have limitations:
- Limited region coverage (most don't have all 33 AWS regions)
- Monthly subscriptions add up if you only need occasional access
- You're trusting a third party with your traffic

With region-proxy, you control the infrastructure, pay only for what you use (~$0.004/hr), and can access any AWS region.

### Q: Isn't this just SSH tunneling? Why a tool?

A: Yes, it's SSH tunneling under the hood. The tool automates the tedious parts:
- Finding the latest Amazon Linux AMI for the region
- Creating/deleting security groups with proper rules
- Managing SSH keys
- Waiting for instance readiness
- Cleaning up resources to avoid forgotten charges

Without automation, this takes ~10 minutes of clicking through AWS console or writing scripts.

### Q: How does this compare to AWS SSM Session Manager?

A: SSM is more secure (no open SSH ports) but requires:
- IAM role configuration
- SSM agent setup
- More complex IAM policies

region-proxy is simpler: just AWS credentials and one command.

### Q: Is this against AWS ToS?

A: No. You're launching EC2 instances for legitimate use (SSH proxy). This is a standard use case. Just don't abuse it for illegal activities or violate the ToS of services you're accessing.

### Q: Why Rust?

A: 
- Single binary distribution (no runtime dependencies)
- Easy cross-compilation for universal macOS binaries
- Great async support with tokio
- AWS SDK for Rust is production-ready

### Q: Why macOS only?

A: macOS integration (networksetup) was my priority since that's what I use. Linux support is planned - the core functionality works, just needs the system proxy configuration part.

### Q: Security concerns?

A: 
- SSH keys are generated per-session and deleted on stop
- Security groups only allow your current IP
- EC2 instances are terminated (not stopped), so no persistent data
- All credentials stay local

### Q: What about IPv6 or split tunneling?

A: Currently IPv4 only, full tunnel. These could be added in future versions.

---

## Timing Tips

Best times to post on HN (Pacific Time):
- Tuesday-Thursday
- 8-10 AM PT (catch East Coast morning + Europe afternoon)
- Avoid weekends and major US holidays

---

## Follow-up Comment (Post after submission)

```
Author here. A few notes:

1. This started as a personal tool for accessing region-restricted APIs during development. I open-sourced it after realizing others might find it useful.

2. Cost breakdown: t4g.nano in Tokyo is $0.0042/hr. If you use it 2 hours/day, that's ~$0.25/month. Compare to VPN subscriptions.

3. The cleanup logic was the trickiest part. If the tool crashes mid-operation, orphaned resources could cost money. I added a `cleanup` command to handle this:

    $ region-proxy cleanup --region ap-northeast-1

4. Future plans:
   - Linux support
   - Multiple simultaneous proxies
   - Connection time limits
   - Cost estimation before start

Feedback and PRs welcome!
```

---

## Alternative Platforms

Consider cross-posting to:

1. **Reddit**
   - r/rust (with [Show Off] or [Project] flair)
   - r/aws
   - r/commandline
   - r/selfhosted

2. **Lobsters** (if you have an invite)

3. **Dev.to** (longer form article)

4. **Twitter/X**
   - Thread format with demo GIF
   - Tag #rustlang #aws #opensource

---

## Success Metrics

Track these after posting:

- [ ] GitHub stars (before/after)
- [ ] Homebrew downloads
- [ ] GitHub issues/PRs from new users
- [ ] HN points and comments
- [ ] Reddit upvotes
