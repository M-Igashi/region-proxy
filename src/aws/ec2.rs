use anyhow::{bail, Context, Result};
use aws_sdk_ec2::types::{
    Filter, InstanceStateName, InstanceType, IpPermission, IpRange, ResourceType, Tag,
    TagSpecification,
};
use aws_sdk_ec2::Client;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info};

const RESOURCE_PREFIX: &str = "region-proxy";

/// EC2 Manager for handling all EC2 operations
pub struct Ec2Manager {
    client: Client,
    #[allow(dead_code)]
    region: String,
}

impl Ec2Manager {
    /// Create a new EC2 manager for the specified region
    pub async fn new(region: &str) -> Result<Self> {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_sdk_ec2::config::Region::new(region.to_string()))
            .load()
            .await;

        let client = Client::new(&config);

        Ok(Self {
            client,
            region: region.to_string(),
        })
    }

    /// Find the latest Amazon Linux 2023 AMI for the given architecture
    pub async fn find_latest_ami(&self, arm: bool) -> Result<String> {
        let arch = if arm { "arm64" } else { "x86_64" };
        info!("Finding latest Amazon Linux 2023 AMI for {}", arch);

        let resp = self
            .client
            .describe_images()
            .owners("amazon")
            .filters(
                Filter::builder()
                    .name("name")
                    .values(format!("al2023-ami-2023.*-{}", arch))
                    .build(),
            )
            .filters(Filter::builder().name("state").values("available").build())
            .filters(Filter::builder().name("architecture").values(arch).build())
            .send()
            .await
            .context("Failed to describe images")?;

        let images = resp.images();
        if images.is_empty() {
            bail!("No Amazon Linux 2023 AMI found for architecture {}", arch);
        }

        // Sort by creation date and get the latest
        let mut images: Vec<_> = images.iter().collect();
        images.sort_by(|a, b| {
            b.creation_date()
                .unwrap_or_default()
                .cmp(a.creation_date().unwrap_or_default())
        });

        let ami_id = images[0]
            .image_id()
            .context("AMI has no image ID")?
            .to_string();

        info!("Found AMI: {}", ami_id);
        Ok(ami_id)
    }

    /// Create a security group for SSH access
    pub async fn create_security_group(&self) -> Result<String> {
        let group_name = format!("{}-{}", RESOURCE_PREFIX, uuid::Uuid::new_v4());
        info!("Creating security group: {}", group_name);

        let resp = self
            .client
            .create_security_group()
            .group_name(&group_name)
            .description("Temporary security group for region-proxy SSH access")
            .tag_specifications(
                TagSpecification::builder()
                    .resource_type(ResourceType::SecurityGroup)
                    .tags(Tag::builder().key("Name").value(&group_name).build())
                    .tags(
                        Tag::builder()
                            .key("CreatedBy")
                            .value(RESOURCE_PREFIX)
                            .build(),
                    )
                    .build(),
            )
            .send()
            .await
            .context("Failed to create security group")?;

        let group_id = resp
            .group_id()
            .context("Security group has no ID")?
            .to_string();

        // Add SSH ingress rule (allow from anywhere for simplicity)
        // In production, you might want to restrict to current IP
        self.client
            .authorize_security_group_ingress()
            .group_id(&group_id)
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

        info!("Created security group: {}", group_id);
        Ok(group_id)
    }

    /// Create a key pair and return the private key
    pub async fn create_key_pair(&self) -> Result<(String, String)> {
        let key_name = format!("{}-{}", RESOURCE_PREFIX, uuid::Uuid::new_v4());
        info!("Creating key pair: {}", key_name);

        let resp = self
            .client
            .create_key_pair()
            .key_name(&key_name)
            .tag_specifications(
                TagSpecification::builder()
                    .resource_type(ResourceType::KeyPair)
                    .tags(
                        Tag::builder()
                            .key("CreatedBy")
                            .value(RESOURCE_PREFIX)
                            .build(),
                    )
                    .build(),
            )
            .send()
            .await
            .context("Failed to create key pair")?;

        let private_key = resp
            .key_material()
            .context("Key pair has no private key")?
            .to_string();

        info!("Created key pair: {}", key_name);
        Ok((key_name, private_key))
    }

    /// Launch an EC2 instance
    pub async fn launch_instance(
        &self,
        ami_id: &str,
        instance_type: &str,
        security_group_id: &str,
        key_name: &str,
    ) -> Result<String> {
        info!("Launching instance: type={}, ami={}", instance_type, ami_id);

        let instance_type = InstanceType::from(instance_type);

        let resp = self
            .client
            .run_instances()
            .image_id(ami_id)
            .instance_type(instance_type)
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
                    .tags(
                        Tag::builder()
                            .key("CreatedBy")
                            .value(RESOURCE_PREFIX)
                            .build(),
                    )
                    .build(),
            )
            .send()
            .await
            .context("Failed to launch instance")?;

        let instance_id = resp
            .instances()
            .first()
            .context("No instance returned")?
            .instance_id()
            .context("Instance has no ID")?
            .to_string();

        info!("Launched instance: {}", instance_id);
        Ok(instance_id)
    }

    /// Wait for instance to be running and return its public IP
    pub async fn wait_for_instance(&self, instance_id: &str) -> Result<String> {
        info!("Waiting for instance {} to be running...", instance_id);

        let max_attempts = 60;
        for attempt in 1..=max_attempts {
            let resp = self
                .client
                .describe_instances()
                .instance_ids(instance_id)
                .send()
                .await
                .context("Failed to describe instance")?;

            let instance = resp
                .reservations()
                .first()
                .and_then(|r| r.instances().first())
                .context("Instance not found")?;

            let state = instance
                .state()
                .and_then(|s| s.name())
                .unwrap_or(&InstanceStateName::Pending);

            debug!(
                "Instance state: {:?} (attempt {}/{})",
                state, attempt, max_attempts
            );

            if *state == InstanceStateName::Running {
                if let Some(ip) = instance.public_ip_address() {
                    info!("Instance is running with IP: {}", ip);

                    // Wait a bit more for SSH to be ready
                    info!("Waiting for SSH to be ready...");
                    sleep(Duration::from_secs(15)).await;

                    return Ok(ip.to_string());
                }
            }

            if *state == InstanceStateName::Terminated || *state == InstanceStateName::ShuttingDown
            {
                bail!("Instance terminated unexpectedly");
            }

            sleep(Duration::from_secs(5)).await;
        }

        bail!("Timeout waiting for instance to be running");
    }

    /// Terminate an instance
    pub async fn terminate_instance(&self, instance_id: &str) -> Result<()> {
        info!("Terminating instance: {}", instance_id);

        self.client
            .terminate_instances()
            .instance_ids(instance_id)
            .send()
            .await
            .context("Failed to terminate instance")?;

        // Wait for termination
        let max_attempts = 30;
        for _ in 1..=max_attempts {
            let resp = self
                .client
                .describe_instances()
                .instance_ids(instance_id)
                .send()
                .await?;

            let state = resp
                .reservations()
                .first()
                .and_then(|r| r.instances().first())
                .and_then(|i| i.state())
                .and_then(|s| s.name());

            if state == Some(&InstanceStateName::Terminated) {
                info!("Instance terminated");
                return Ok(());
            }

            sleep(Duration::from_secs(2)).await;
        }

        Ok(())
    }

    /// Delete a security group
    pub async fn delete_security_group(&self, group_id: &str) -> Result<()> {
        info!("Deleting security group: {}", group_id);

        // Retry a few times as the security group might still be in use
        for attempt in 1..=5 {
            match self
                .client
                .delete_security_group()
                .group_id(group_id)
                .send()
                .await
            {
                Ok(_) => {
                    info!("Deleted security group");
                    return Ok(());
                }
                Err(e) => {
                    if attempt < 5 {
                        debug!("Retrying security group deletion: {}", e);
                        sleep(Duration::from_secs(5)).await;
                    } else {
                        return Err(e).context("Failed to delete security group");
                    }
                }
            }
        }

        Ok(())
    }

    /// Delete a key pair
    pub async fn delete_key_pair(&self, key_name: &str) -> Result<()> {
        info!("Deleting key pair: {}", key_name);

        self.client
            .delete_key_pair()
            .key_name(key_name)
            .send()
            .await
            .context("Failed to delete key pair")?;

        info!("Deleted key pair");
        Ok(())
    }

    /// Find orphaned resources created by region-proxy
    pub async fn find_orphaned_resources(&self) -> Result<OrphanedResources> {
        let mut orphaned = OrphanedResources::default();

        // Find instances
        let resp = self
            .client
            .describe_instances()
            .filters(
                Filter::builder()
                    .name("tag:CreatedBy")
                    .values(RESOURCE_PREFIX)
                    .build(),
            )
            .filters(
                Filter::builder()
                    .name("instance-state-name")
                    .values("running")
                    .values("pending")
                    .values("stopping")
                    .values("stopped")
                    .build(),
            )
            .send()
            .await?;

        for reservation in resp.reservations() {
            for instance in reservation.instances() {
                if let Some(id) = instance.instance_id() {
                    orphaned.instance_ids.push(id.to_string());
                }
            }
        }

        // Find security groups
        let resp = self
            .client
            .describe_security_groups()
            .filters(
                Filter::builder()
                    .name("tag:CreatedBy")
                    .values(RESOURCE_PREFIX)
                    .build(),
            )
            .send()
            .await?;

        for sg in resp.security_groups() {
            if let Some(id) = sg.group_id() {
                orphaned.security_group_ids.push(id.to_string());
            }
        }

        // Find key pairs
        let resp = self
            .client
            .describe_key_pairs()
            .filters(
                Filter::builder()
                    .name("tag:CreatedBy")
                    .values(RESOURCE_PREFIX)
                    .build(),
            )
            .send()
            .await?;

        for kp in resp.key_pairs() {
            if let Some(name) = kp.key_name() {
                orphaned.key_pair_names.push(name.to_string());
            }
        }

        Ok(orphaned)
    }
}

#[derive(Debug, Default)]
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
