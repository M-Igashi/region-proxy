use clap::{Parser, Subcommand, ValueEnum};

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
        /// AWS region (e.g., ap-northeast-1, us-west-2). Uses default if not specified.
        #[arg(short, long)]
        region: Option<String>,

        /// Local port for SOCKS proxy (default: 1080)
        #[arg(short, long, value_parser = clap::value_parser!(u16).range(1..))]
        port: Option<u16>,

        /// EC2 instance type (default: t4g.nano)
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
        /// Show regions in a table
        #[arg(short, long)]
        detailed: bool,
    },

    /// Cleanup orphaned AWS resources
    Cleanup {
        /// Specific region to cleanup (default: all regions)
        #[arg(short, long)]
        region: Option<String>,
    },

    /// Manage configuration and preferences
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Show current configuration
    Show,

    /// Set default region
    SetRegion {
        /// AWS region code (e.g., ap-northeast-1)
        region: String,
    },

    /// Set default port
    SetPort {
        /// Local port number (e.g., 1080)
        #[arg(value_parser = clap::value_parser!(u16).range(1..))]
        port: u16,
    },

    /// Set default instance type
    SetInstanceType {
        /// EC2 instance type (e.g., t4g.nano, t3.micro)
        instance_type: String,
    },

    /// Set whether to skip system proxy configuration
    SetNoSystemProxy {
        /// true or false
        #[arg(action = clap::ArgAction::Set)]
        value: bool,
    },

    /// Clear a specific configuration option
    Unset {
        /// Option to clear
        #[arg(value_enum)]
        option: UnsetOption,
    },

    /// Clear all configuration
    Reset,
}

#[derive(ValueEnum, Clone, Copy)]
#[value(rename_all = "kebab-case")]
pub enum UnsetOption {
    Region,
    Port,
    InstanceType,
    NoSystemProxy,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn test_cli_command_structure() {
        Cli::command().debug_assert();
    }
}
