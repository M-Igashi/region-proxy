use anyhow::{Context, Result};
use aws_config::SdkConfig;
use aws_sdk_ec2::client::Waiters;
use aws_sdk_ec2::config::Region;
use aws_sdk_ec2::error::ProvideErrorMetadata;
use aws_sdk_ec2::types::{
    Filter, InstanceType, IpPermission, IpRange, ResourceType, Tag, TagSpecification,
};
use aws_sdk_ec2::Client;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info};

const RESOURCE_PREFIX: &str = "region-proxy";
const CREATED_BY_TAG: &str = "CreatedBy";

pub async fn load_config(region: &str) -> SdkConfig {
    aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(Region::new(region.to_string()))
        .load()
        .await
}

/// Graviton families have a `g` right after the generation number (t4g, m7gd, c7gn, ...)
fn is_arm_instance_type(instance_type: &str) -> bool {
    let family = instance_type.split('.').next().unwrap_or_default();
    let mut chars = family.chars().skip_while(|c| !c.is_ascii_digit());
    chars.find(|c| !c.is_ascii_digit()) == Some('g')
}

/// EC2 resolves the latest Amazon Linux 2023 AMI from the AWS public SSM parameter at launch
fn latest_ami(instance_type: &str) -> String {
    let arch = if is_arm_instance_type(instance_type) {
        "arm64"
    } else {
        "x86_64"
    };
    format!(
        "resolve:ssm:/aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-{}",
        arch
    )
}

fn created_by_tag() -> Tag {
    Tag::builder()
        .key(CREATED_BY_TAG)
        .value(RESOURCE_PREFIX)
        .build()
}

fn created_by_filter() -> Filter {
    Filter::builder()
        .name(format!("tag:{}", CREATED_BY_TAG))
        .values(RESOURCE_PREFIX)
        .build()
}

pub struct Ec2Manager {
    client: Client,
}

impl Ec2Manager {
    pub fn new(config: &SdkConfig, region: &str) -> Self {
        let conf = aws_sdk_ec2::config::Builder::from(config)
            .region(Region::new(region.to_string()))
            .build();
        Self {
            client: Client::from_conf(conf),
        }
    }

    pub async fn create_security_group(&self) -> Result<String> {
        let group_name = format!("{}-{}", RESOURCE_PREFIX, uuid::Uuid::new_v4());
        debug!("Creating security group: {}", group_name);

        let resp = self
            .client
            .create_security_group()
            .group_name(&group_name)
            .description("Temporary security group for region-proxy SSH access")
            .tag_specifications(
                TagSpecification::builder()
                    .resource_type(ResourceType::SecurityGroup)
                    .tags(Tag::builder().key("Name").value(&group_name).build())
                    .tags(created_by_tag())
                    .build(),
            )
            .send()
            .await
            .context("Failed to create security group")?;

        let group_id = resp
            .group_id()
            .context("Security group has no ID")?
            .to_string();

        debug!("Created security group: {}", group_id);
        Ok(group_id)
    }

    pub async fn allow_ssh(&self, group_id: &str) -> Result<()> {
        self.client
            .authorize_security_group_ingress()
            .group_id(group_id)
            .ip_permissions(
                IpPermission::builder()
                    .ip_protocol("tcp")
                    .from_port(22)
                    .to_port(22)
                    .ip_ranges(IpRange::builder().cidr_ip("0.0.0.0/0").build())
                    .build(),
            )
            .send()
            .await
            .context("Failed to add SSH ingress rule")?;

        Ok(())
    }

    pub async fn create_key_pair(&self) -> Result<(String, String)> {
        let key_name = format!("{}-{}", RESOURCE_PREFIX, uuid::Uuid::new_v4());
        debug!("Creating key pair: {}", key_name);

        let resp = self
            .client
            .create_key_pair()
            .key_name(&key_name)
            .tag_specifications(
                TagSpecification::builder()
                    .resource_type(ResourceType::KeyPair)
                    .tags(created_by_tag())
                    .build(),
            )
            .send()
            .await
            .context("Failed to create key pair")?;

        let private_key = resp
            .key_material()
            .context("Key pair has no private key")?
            .to_string();

        debug!("Created key pair: {}", key_name);
        Ok((key_name, private_key))
    }

    pub async fn launch_instance(
        &self,
        instance_type: &str,
        security_group_id: &str,
        key_name: &str,
    ) -> Result<String> {
        let image_id = latest_ami(instance_type);
        debug!(
            "Launching instance: type={}, image={}",
            instance_type, image_id
        );

        let result = self
            .client
            .run_instances()
            .image_id(image_id)
            .instance_type(InstanceType::from(instance_type))
            .min_count(1)
            .max_count(1)
            .security_group_ids(security_group_id)
            .key_name(key_name)
            .tag_specifications(
                TagSpecification::builder()
                    .resource_type(ResourceType::Instance)
                    .tags(
                        Tag::builder()
                            .key("Name")
                            .value(format!("{}-instance", RESOURCE_PREFIX))
                            .build(),
                    )
                    .tags(created_by_tag())
                    .build(),
            )
            .send()
            .await;

        let resp = match result {
            Err(e) if e.code() == Some("SsmAccessDenied") => {
                return Err(e).context(
                    "Missing ssm:GetParameters permission to look up the AMI. \
                     Update your IAM policy: https://github.com/M-Igashi/region-proxy#upgrading-from-v130-or-earlier",
                );
            }
            other => other.context("Failed to launch instance")?,
        };

        let instance = resp.instances().first().context("No instance returned")?;
        let instance_id = instance
            .instance_id()
            .context("Instance has no ID")?
            .to_string();

        debug!(
            "Launched instance: {} (AMI {})",
            instance_id,
            instance.image_id().unwrap_or("unknown")
        );
        Ok(instance_id)
    }

    /// Wait until the instance is running and return its public IP
    pub async fn wait_for_instance(&self, instance_id: &str) -> Result<String> {
        debug!("Waiting for instance {} to be running...", instance_id);

        let resp = self
            .client
            .wait_until_instance_running()
            .instance_ids(instance_id)
            .wait(Duration::from_secs(300))
            .await
            .context("Instance did not reach running state")?
            .into_result()
            .context("Failed to describe instance")?;

        let ip = resp
            .reservations()
            .first()
            .and_then(|r| r.instances().first())
            .and_then(|i| i.public_ip_address())
            .context("Instance has no public IP")?
            .to_string();

        debug!("Instance is running with IP: {}", ip);
        Ok(ip)
    }

    /// Request termination without waiting; `delete_security_group` retries until the instance is gone
    pub async fn terminate_instances(&self, instance_ids: &[String]) -> Result<()> {
        debug!("Terminating instance(s): {}", instance_ids.join(", "));

        self.client
            .terminate_instances()
            .set_instance_ids(Some(instance_ids.to_vec()))
            .send()
            .await
            .context("Failed to terminate instances")?;

        Ok(())
    }

    pub async fn delete_security_group(&self, group_id: &str) -> Result<()> {
        debug!("Deleting security group: {}", group_id);

        const MAX_ATTEMPTS: u32 = 36; // ~3 min at 5s intervals
        let mut attempt = 0;
        loop {
            attempt += 1;
            let result = self
                .client
                .delete_security_group()
                .group_id(group_id)
                .send()
                .await;

            let Err(e) = result else {
                debug!("Deleted security group");
                return Ok(());
            };

            match e.code() {
                Some("InvalidGroup.NotFound") => {
                    info!("Security group already deleted");
                    return Ok(());
                }
                Some("DependencyViolation") if attempt < MAX_ATTEMPTS => {
                    debug!("Security group still in use, retrying: {}", e);
                    sleep(Duration::from_secs(5)).await;
                }
                _ => return Err(e).context("Failed to delete security group"),
            }
        }
    }

    pub async fn delete_key_pair(&self, key_name: &str) -> Result<()> {
        debug!("Deleting key pair: {}", key_name);

        self.client
            .delete_key_pair()
            .key_name(key_name)
            .send()
            .await
            .context("Failed to delete key pair")?;

        debug!("Deleted key pair");
        Ok(())
    }

    pub async fn find_orphaned_resources(&self) -> Result<OrphanedResources> {
        let instances_fut = self
            .client
            .describe_instances()
            .filters(created_by_filter())
            .filters(
                Filter::builder()
                    .name("instance-state-name")
                    .values("running")
                    .values("pending")
                    .values("stopping")
                    .values("stopped")
                    .build(),
            )
            .send();

        let sgs_fut = self
            .client
            .describe_security_groups()
            .filters(created_by_filter())
            .send();

        let kps_fut = self
            .client
            .describe_key_pairs()
            .filters(created_by_filter())
            .send();

        let (instances_resp, sgs_resp, kps_resp) = tokio::try_join!(
            async { instances_fut.await.context("Failed to describe instances") },
            async { sgs_fut.await.context("Failed to describe security groups") },
            async { kps_fut.await.context("Failed to describe key pairs") },
        )?;

        Ok(OrphanedResources {
            instance_ids: instances_resp
                .reservations()
                .iter()
                .flat_map(|r| r.instances())
                .filter_map(|i| i.instance_id())
                .map(str::to_string)
                .collect(),
            security_group_ids: sgs_resp
                .security_groups()
                .iter()
                .filter_map(|sg| sg.group_id())
                .map(str::to_string)
                .collect(),
            key_pair_names: kps_resp
                .key_pairs()
                .iter()
                .filter_map(|kp| kp.key_name())
                .map(str::to_string)
                .collect(),
        })
    }
}

pub struct OrphanedResources {
    pub instance_ids: Vec<String>,
    pub security_group_ids: Vec<String>,
    pub key_pair_names: Vec<String>,
}

impl OrphanedResources {
    pub fn is_empty(&self) -> bool {
        self.instance_ids.is_empty()
            && self.security_group_ids.is_empty()
            && self.key_pair_names.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_arm_instance_type() {
        for arm in [
            "t4g.nano",
            "m7g.large",
            "c7gn.medium",
            "m6gd.xlarge",
            "x2gd.large",
        ] {
            assert!(is_arm_instance_type(arm), "{}", arm);
        }
        for x86 in [
            "t3.nano",
            "t3a.micro",
            "m7i.large",
            "g4dn.xlarge",
            "c5n.large",
            "",
        ] {
            assert!(!is_arm_instance_type(x86), "{}", x86);
        }
    }
}
