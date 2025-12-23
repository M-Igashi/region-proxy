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
        /// AWS region (e.g., ap-northeast-1, us-west-2). Uses default if not specified.
        #[arg(short, long)]
        region: Option<String>,

        /// Local port for SOCKS proxy (default: 1080)
        #[arg(short, long)]
        port: Option<u16>,

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
        value: String,
    },

    /// Clear a specific configuration option
    Unset {
        /// Option to clear: region, port, instance-type, no-system-proxy
        option: String,
    },

    /// Clear all configuration
    Reset,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn test_cli_parse_start_with_region() {
        let cli = Cli::parse_from(["region-proxy", "start", "--region", "ap-northeast-1"]);
        match cli.command {
            Commands::Start {
                region,
                port,
                instance_type,
                no_system_proxy,
            } => {
                assert_eq!(region, Some("ap-northeast-1".to_string()));
                assert!(port.is_none());
                assert!(instance_type.is_none());
                assert!(!no_system_proxy);
            }
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_cli_parse_start_without_region() {
        let cli = Cli::parse_from(["region-proxy", "start"]);
        match cli.command {
            Commands::Start {
                region,
                port,
                instance_type,
                no_system_proxy,
            } => {
                assert!(region.is_none());
                assert!(port.is_none());
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
                assert_eq!(region, Some("us-west-2".to_string()));
                assert_eq!(port, Some(8080));
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
    fn test_cli_parse_config_show() {
        let cli = Cli::parse_from(["region-proxy", "config", "show"]);
        match cli.command {
            Commands::Config { action } => {
                assert!(matches!(action, ConfigAction::Show));
            }
            _ => panic!("Expected Config command"),
        }
    }

    #[test]
    fn test_cli_parse_config_set_region() {
        let cli = Cli::parse_from(["region-proxy", "config", "set-region", "ap-northeast-1"]);
        match cli.command {
            Commands::Config { action } => match action {
                ConfigAction::SetRegion { region } => {
                    assert_eq!(region, "ap-northeast-1");
                }
                _ => panic!("Expected SetRegion action"),
            },
            _ => panic!("Expected Config command"),
        }
    }

    #[test]
    fn test_cli_parse_config_set_port() {
        let cli = Cli::parse_from(["region-proxy", "config", "set-port", "8080"]);
        match cli.command {
            Commands::Config { action } => match action {
                ConfigAction::SetPort { port } => {
                    assert_eq!(port, 8080);
                }
                _ => panic!("Expected SetPort action"),
            },
            _ => panic!("Expected Config command"),
        }
    }

    #[test]
    fn test_cli_parse_config_reset() {
        let cli = Cli::parse_from(["region-proxy", "config", "reset"]);
        match cli.command {
            Commands::Config { action } => {
                assert!(matches!(action, ConfigAction::Reset));
            }
            _ => panic!("Expected Config command"),
        }
    }

    #[test]
    fn test_cli_parse_config_unset() {
        let cli = Cli::parse_from(["region-proxy", "config", "unset", "region"]);
        match cli.command {
            Commands::Config { action } => match action {
                ConfigAction::Unset { option } => {
                    assert_eq!(option, "region");
                }
                _ => panic!("Expected Unset action"),
            },
            _ => panic!("Expected Config command"),
        }
    }

    #[test]
    fn test_cli_command_structure() {
        // Verify the CLI structure is valid
        Cli::command().debug_assert();
    }
}
