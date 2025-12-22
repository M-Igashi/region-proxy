use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "region-proxy")]
#[command(author = "Masanari Higashi")]
#[command(version)]
#[command(about = "Create a SOCKS proxy through AWS EC2 in any region", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start a proxy in the specified AWS region
    Start {
        /// AWS region (e.g., ap-northeast-1, us-west-2)
        #[arg(short, long)]
        region: String,

        /// Local port for SOCKS proxy (default: 1080)
        #[arg(short, long, default_value = "1080")]
        port: u16,

        /// EC2 instance type (default: t4g.nano for ARM regions, t3.nano otherwise)
        #[arg(short, long)]
        instance_type: Option<String>,

        /// Skip macOS system proxy configuration
        #[arg(long)]
        no_system_proxy: bool,
    },

    /// Stop the running proxy and cleanup AWS resources
    Stop {
        /// Force cleanup even if some operations fail
        #[arg(short, long)]
        force: bool,
    },

    /// Show the current proxy status
    Status,

    /// List available AWS regions
    ListRegions {
        /// Show only regions with description
        #[arg(short, long)]
        detailed: bool,
    },

    /// Cleanup orphaned AWS resources
    Cleanup {
        /// Specific region to cleanup (default: all regions)
        #[arg(short, long)]
        region: Option<String>,
    },
}
