#[derive(Debug, Clone)]
pub struct RegionInfo {
    pub code: &'static str,
    pub name: &'static str,
}

pub const REGIONS: &[RegionInfo] = &[
    // Asia Pacific
    RegionInfo { code: "ap-northeast-1", name: "Tokyo" },
    RegionInfo { code: "ap-northeast-2", name: "Seoul" },
    RegionInfo { code: "ap-northeast-3", name: "Osaka" },
    RegionInfo { code: "ap-southeast-1", name: "Singapore" },
    RegionInfo { code: "ap-southeast-2", name: "Sydney" },
    RegionInfo { code: "ap-south-1", name: "Mumbai" },
    // US
    RegionInfo { code: "us-east-1", name: "N. Virginia" },
    RegionInfo { code: "us-east-2", name: "Ohio" },
    RegionInfo { code: "us-west-1", name: "N. California" },
    RegionInfo { code: "us-west-2", name: "Oregon" },
    // Europe
    RegionInfo { code: "eu-west-1", name: "Ireland" },
    RegionInfo { code: "eu-west-2", name: "London" },
    RegionInfo { code: "eu-west-3", name: "Paris" },
    RegionInfo { code: "eu-central-1", name: "Frankfurt" },
    RegionInfo { code: "eu-north-1", name: "Stockholm" },
    // South America
    RegionInfo { code: "sa-east-1", name: "São Paulo" },
    // Canada
    RegionInfo { code: "ca-central-1", name: "Canada" },
];

impl RegionInfo {
    pub fn default_instance_type(&self) -> &'static str {
        "t4g.nano"
    }
}

pub fn find_region(code: &str) -> Option<&'static RegionInfo> {
    REGIONS.iter().find(|r| r.code == code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_region_tokyo() {
        let region = find_region("ap-northeast-1").unwrap();
        assert_eq!(region.code, "ap-northeast-1");
        assert_eq!(region.name, "Tokyo");
    }

    #[test]
    fn test_find_region_oregon() {
        let region = find_region("us-west-2").unwrap();
        assert_eq!(region.code, "us-west-2");
        assert_eq!(region.name, "Oregon");
    }

    #[test]
    fn test_find_region_invalid() {
        assert!(find_region("invalid-region").is_none());
    }

    #[test]
    fn test_find_region_empty() {
        assert!(find_region("").is_none());
    }

    #[test]
    fn test_default_instance_type_arm() {
        let region = find_region("ap-northeast-1").unwrap();
        assert_eq!(region.default_instance_type(), "t4g.nano");
    }

    #[test]
    fn test_regions_not_empty() {
        assert!(!REGIONS.is_empty());
        assert!(REGIONS.len() >= 17);
    }

    #[test]
    fn test_all_regions_have_valid_codes() {
        for region in REGIONS {
            assert!(!region.code.is_empty());
            assert!(!region.name.is_empty());
            assert!(region.code.contains('-'));
        }
    }
}
