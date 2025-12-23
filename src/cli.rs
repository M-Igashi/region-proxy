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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn test_cli_parse_start() {
        let cli = Cli::parse_from(["region-proxy", "start", "--region", "ap-northeast-1"]);
        match cli.command {
            Commands::Start {
                region,
                port,
                instance_type,
                no_system_proxy,
            } => {
                assert_eq!(region, "ap-northeast-1");
                assert_eq!(port, 1080); // default
                assert!(instance_type.is_none());
                assert!(!no_system_proxy);
            }
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_cli_parse_start_with_options() {
        let cli = Cli::parse_from([
            "region-proxy",
            "start",
            "--region",
            "us-west-2",
            "--port",
            "8080",
            "--instance-type",
            "t3.micro",
            "--no-system-proxy",
        ]);
        match cli.command {
            Commands::Start {
                region,
                port,
                instance_type,
                no_system_proxy,
            } => {
                assert_eq!(region, "us-west-2");
                assert_eq!(port, 8080);
                assert_eq!(instance_type, Some("t3.micro".to_string()));
                assert!(no_system_proxy);
            }
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_cli_parse_stop() {
        let cli = Cli::parse_from(["region-proxy", "stop"]);
        match cli.command {
            Commands::Stop { force } => {
                assert!(!force);
            }
            _ => panic!("Expected Stop command"),
        }
    }

    #[test]
    fn test_cli_parse_stop_force() {
        let cli = Cli::parse_from(["region-proxy", "stop", "--force"]);
        match cli.command {
            Commands::Stop { force } => {
                assert!(force);
            }
            _ => panic!("Expected Stop command"),
        }
    }

    #[test]
    fn test_cli_parse_status() {
        let cli = Cli::parse_from(["region-proxy", "status"]);
        assert!(matches!(cli.command, Commands::Status));
    }

    #[test]
    fn test_cli_parse_list_regions() {
        let cli = Cli::parse_from(["region-proxy", "list-regions"]);
        match cli.command {
            Commands::ListRegions { detailed } => {
                assert!(!detailed);
            }
            _ => panic!("Expected ListRegions command"),
        }
    }

    #[test]
    fn test_cli_parse_list_regions_detailed() {
        let cli = Cli::parse_from(["region-proxy", "list-regions", "--detailed"]);
        match cli.command {
            Commands::ListRegions { detailed } => {
                assert!(detailed);
            }
            _ => panic!("Expected ListRegions command"),
        }
    }

    #[test]
    fn test_cli_parse_cleanup() {
        let cli = Cli::parse_from(["region-proxy", "cleanup"]);
        match cli.command {
            Commands::Cleanup { region } => {
                assert!(region.is_none());
            }
            _ => panic!("Expected Cleanup command"),
        }
    }

    #[test]
    fn test_cli_parse_cleanup_with_region() {
        let cli = Cli::parse_from(["region-proxy", "cleanup", "--region", "eu-west-1"]);
        match cli.command {
            Commands::Cleanup { region } => {
                assert_eq!(region, Some("eu-west-1".to_string()));
            }
            _ => panic!("Expected Cleanup command"),
        }
    }

    #[test]
    fn test_cli_verbose_flag() {
        let cli = Cli::parse_from(["region-proxy", "-v", "status"]);
        assert!(cli.verbose);
    }

    #[test]
    fn test_cli_command_structure() {
        // Verify the CLI structure is valid
        Cli::command().debug_assert();
    }
}
