/// AWS Region information
#[derive(Debug, Clone)]
pub struct RegionInfo {
    pub code: &'static str,
    pub name: &'static str,
    pub supports_arm: bool,
}

/// List of commonly used AWS regions
pub const REGIONS: &[RegionInfo] = &[
    // Asia Pacific
    RegionInfo { code: "ap-northeast-1", name: "Tokyo", supports_arm: true },
    RegionInfo { code: "ap-northeast-2", name: "Seoul", supports_arm: true },
    RegionInfo { code: "ap-northeast-3", name: "Osaka", supports_arm: true },
    RegionInfo { code: "ap-southeast-1", name: "Singapore", supports_arm: true },
    RegionInfo { code: "ap-southeast-2", name: "Sydney", supports_arm: true },
    RegionInfo { code: "ap-south-1", name: "Mumbai", supports_arm: true },
    
    // US
    RegionInfo { code: "us-east-1", name: "N. Virginia", supports_arm: true },
    RegionInfo { code: "us-east-2", name: "Ohio", supports_arm: true },
    RegionInfo { code: "us-west-1", name: "N. California", supports_arm: true },
    RegionInfo { code: "us-west-2", name: "Oregon", supports_arm: true },
    
    // Europe
    RegionInfo { code: "eu-west-1", name: "Ireland", supports_arm: true },
    RegionInfo { code: "eu-west-2", name: "London", supports_arm: true },
    RegionInfo { code: "eu-west-3", name: "Paris", supports_arm: true },
    RegionInfo { code: "eu-central-1", name: "Frankfurt", supports_arm: true },
    RegionInfo { code: "eu-north-1", name: "Stockholm", supports_arm: true },
    
    // South America
    RegionInfo { code: "sa-east-1", name: "São Paulo", supports_arm: true },
    
    // Canada
    RegionInfo { code: "ca-central-1", name: "Canada", supports_arm: true },
];

impl RegionInfo {
    /// Get the default instance type for this region
    pub fn default_instance_type(&self) -> &'static str {
        if self.supports_arm {
            "t4g.nano"
        } else {
            "t3.nano"
        }
    }
}

/// Find a region by its code
pub fn find_region(code: &str) -> Option<&'static RegionInfo> {
    REGIONS.iter().find(|r| r.code == code)
}

/// Check if a region code is valid
pub fn is_valid_region(code: &str) -> bool {
    find_region(code).is_some()
}
