// Generated with https://app.quicktype.io/ with the input from `nomad job run -output
// my-tmp-file.nomad`. See https://www.nomadproject.io/api-docs/json-jobs
//
// * Replace `env:` with `HashMap`
// * Implement `Default` for all structs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Job {
	#[serde(rename = "Region")]
	pub region: Option<serde_json::Value>,
	#[serde(rename = "Namespace")]
	pub namespace: Option<serde_json::Value>,
	#[serde(rename = "ID")]
	pub id: Option<String>,
	#[serde(rename = "Name")]
	pub name: Option<String>,
	#[serde(rename = "Type")]
	pub job_type: Option<String>,
	#[serde(rename = "Priority")]
	pub priority: Option<i64>,
	#[serde(rename = "AllAtOnce")]
	pub all_at_once: Option<serde_json::Value>,
	#[serde(rename = "Datacenters")]
	pub datacenters: Option<Vec<String>>,
	#[serde(rename = "Constraints")]
	pub constraints: Option<serde_json::Value>,
	#[serde(rename = "Affinities")]
	pub affinities: Option<serde_json::Value>,
	#[serde(rename = "TaskGroups")]
	pub task_groups: Option<Vec<TaskGroup>>,
	#[serde(rename = "Update")]
	pub update: Option<Update>,
	#[serde(rename = "Multiregion")]
	pub multiregion: Option<serde_json::Value>,
	#[serde(rename = "Spreads")]
	pub spreads: Option<serde_json::Value>,
	#[serde(rename = "Periodic")]
	pub periodic: Option<serde_json::Value>,
	#[serde(rename = "ParameterizedJob")]
	pub parameterized_job: Option<serde_json::Value>,
	#[serde(rename = "Reschedule")]
	pub reschedule: Option<serde_json::Value>,
	#[serde(rename = "Migrate")]
	pub migrate: Option<serde_json::Value>,
	#[serde(rename = "Meta")]
	pub meta: Option<serde_json::Value>,
	#[serde(rename = "ConsulToken")]
	pub consul_token: Option<serde_json::Value>,
	#[serde(rename = "VaultToken")]
	pub vault_token: Option<serde_json::Value>,
	#[serde(rename = "Stop")]
	pub stop: Option<serde_json::Value>,
	#[serde(rename = "ParentID")]
	pub parent_id: Option<serde_json::Value>,
	#[serde(rename = "Dispatched")]
	pub dispatched: Option<bool>,
	#[serde(rename = "Payload")]
	pub payload: Option<serde_json::Value>,
	#[serde(rename = "VaultNamespace")]
	pub vault_namespace: Option<serde_json::Value>,
	#[serde(rename = "NomadTokenID")]
	pub nomad_token_id: Option<serde_json::Value>,
	#[serde(rename = "Status")]
	pub status: Option<serde_json::Value>,
	#[serde(rename = "StatusDescription")]
	pub status_description: Option<serde_json::Value>,
	#[serde(rename = "Stable")]
	pub stable: Option<serde_json::Value>,
	#[serde(rename = "Version")]
	pub version: Option<serde_json::Value>,
	#[serde(rename = "SubmitTime")]
	pub submit_time: Option<serde_json::Value>,
	#[serde(rename = "CreateIndex")]
	pub create_index: Option<serde_json::Value>,
	#[serde(rename = "ModifyIndex")]
	pub modify_index: Option<serde_json::Value>,
	#[serde(rename = "JobModifyIndex")]
	pub job_modify_index: Option<serde_json::Value>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TaskGroup {
	#[serde(rename = "Name")]
	pub name: Option<String>,
	#[serde(rename = "Count")]
	pub count: Option<i64>,
	#[serde(rename = "Constraints")]
	pub constraints: Option<serde_json::Value>,
	#[serde(rename = "Affinities")]
	pub affinities: Option<serde_json::Value>,
	#[serde(rename = "Tasks")]
	pub tasks: Option<Vec<Task>>,
	#[serde(rename = "Spreads")]
	pub spreads: Option<serde_json::Value>,
	#[serde(rename = "Volumes")]
	pub volumes: Option<serde_json::Value>,
	#[serde(rename = "RestartPolicy")]
	pub restart_policy: Option<RestartPolicy>,
	#[serde(rename = "ReschedulePolicy")]
	pub reschedule_policy: Option<ReschedulePolicy>,
	#[serde(rename = "EphemeralDisk")]
	pub ephemeral_disk: Option<serde_json::Value>,
	#[serde(rename = "Update")]
	pub update: Option<Update>,
	#[serde(rename = "Migrate")]
	pub migrate: Option<serde_json::Value>,
	#[serde(rename = "Networks")]
	pub networks: Option<Vec<Network>>,
	#[serde(rename = "Meta")]
	pub meta: Option<serde_json::Value>,
	#[serde(rename = "Services")]
	pub services: Option<Vec<Service>>,
	#[serde(rename = "ShutdownDelay")]
	pub shutdown_delay: Option<serde_json::Value>,
	#[serde(rename = "StopAfterClientDisconnect")]
	pub stop_after_client_disconnect: Option<serde_json::Value>,
	#[serde(rename = "Scaling")]
	pub scaling: Option<serde_json::Value>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Update {
	#[serde(rename = "HealthCheck")]
	pub health_check: Option<String>,
	#[serde(rename = "MaxParallel")]
	pub max_parallel: Option<i64>,
	#[serde(rename = "Stagger")]
	pub stagger: Option<i64>,
	#[serde(rename = "MinHealthyTime")]
	pub min_healthy_time: Option<i64>,
	#[serde(rename = "HealthyDeadline")]
	pub healthy_deadline: Option<i64>,
	#[serde(rename = "ProgressDeadline")]
	pub progress_deadline: Option<i64>,
	#[serde(rename = "AutoRevert")]
	pub auto_revert: Option<bool>,
	#[serde(rename = "Canary")]
	pub canary: Option<i64>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Network {
	#[serde(rename = "Mode")]
	pub mode: Option<String>,
	#[serde(rename = "Device")]
	pub device: Option<String>,
	#[serde(rename = "CIDR")]
	pub cidr: Option<String>,
	#[serde(rename = "IP")]
	pub ip: Option<String>,
	#[serde(rename = "DNS")]
	pub dns: Option<serde_json::Value>,
	#[serde(rename = "ReservedPorts")]
	pub reserved_ports: Option<serde_json::Value>,
	#[serde(rename = "DynamicPorts")]
	pub dynamic_ports: Option<Vec<DynamicPort>>,
	#[serde(rename = "MBits")]
	pub m_bits: Option<serde_json::Value>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DynamicPort {
	#[serde(rename = "Label")]
	pub label: Option<String>,
	#[serde(rename = "Value")]
	pub value: Option<i64>,
	#[serde(rename = "To")]
	pub to: Option<i64>,
	#[serde(rename = "HostNetwork")]
	pub host_network: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RestartPolicy {
	#[serde(rename = "Interval")]
	pub interval: Option<i64>,
	#[serde(rename = "Attempts")]
	pub attempts: Option<i64>,
	#[serde(rename = "Delay")]
	pub delay: Option<i64>,
	#[serde(rename = "Mode")]
	pub mode: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ReschedulePolicy {
	#[serde(rename = "Attempts")]
	pub attempts: Option<i64>,
	#[serde(rename = "Delay")]
	pub delay: Option<i64>,
	#[serde(rename = "DelayFunction")]
	pub delay_function: Option<String>,
	#[serde(rename = "Interval")]
	pub interval: Option<i64>,
	#[serde(rename = "MaxDelay")]
	pub max_delay: Option<i64>,
	#[serde(rename = "Unlimited")]
	pub unlimited: Option<bool>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Service {
	#[serde(rename = "Id")]
	pub id: Option<String>,
	#[serde(rename = "Name")]
	pub name: Option<String>,
	#[serde(rename = "Tags")]
	pub tags: Option<Vec<String>>,
	#[serde(rename = "CanaryTags")]
	pub canary_tags: Option<serde_json::Value>,
	#[serde(rename = "EnableTagOverride")]
	pub enable_tag_override: Option<bool>,
	#[serde(rename = "PortLabel")]
	pub port_label: Option<String>,
	#[serde(rename = "AddressMode")]
	pub address_mode: Option<String>,
	#[serde(rename = "Checks")]
	pub checks: Option<Vec<Check>>,
	#[serde(rename = "CheckRestart")]
	pub check_restart: Option<CheckRestart>,
	#[serde(rename = "Connect")]
	pub connect: Option<Connect>,
	#[serde(rename = "Meta")]
	pub meta: Option<serde_json::Value>,
	#[serde(rename = "CanaryMeta")]
	pub canary_meta: Option<serde_json::Value>,
	#[serde(rename = "TaskName")]
	pub task_name: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CheckRestart {
	#[serde(rename = "Limit")]
	pub limit: Option<i64>,
	#[serde(rename = "Grace")]
	pub grace: Option<i64>,
	#[serde(rename = "IgnoreWarnings")]
	pub ignore_warnings: Option<bool>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Check {
	#[serde(rename = "Id")]
	pub id: Option<String>,
	#[serde(rename = "Name")]
	pub name: Option<String>,
	#[serde(rename = "Type")]
	pub check_type: Option<String>,
	#[serde(rename = "Command")]
	pub command: Option<String>,
	#[serde(rename = "Args")]
	pub args: Option<serde_json::Value>,
	#[serde(rename = "Path")]
	pub path: Option<String>,
	#[serde(rename = "Protocol")]
	pub protocol: Option<String>,
	#[serde(rename = "PortLabel")]
	pub port_label: Option<String>,
	#[serde(rename = "Expose")]
	pub expose: Option<bool>,
	#[serde(rename = "AddressMode")]
	pub address_mode: Option<String>,
	#[serde(rename = "Interval")]
	pub interval: Option<i64>,
	#[serde(rename = "Timeout")]
	pub timeout: Option<i64>,
	#[serde(rename = "InitialStatus")]
	pub initial_status: Option<String>,
	#[serde(rename = "TLSSkipVerify")]
	pub tls_skip_verify: Option<bool>,
	#[serde(rename = "Header")]
	pub header: Option<serde_json::Value>,
	#[serde(rename = "Method")]
	pub method: Option<String>,
	#[serde(rename = "CheckRestart")]
	pub check_restart: Option<CheckRestart>,
	#[serde(rename = "OnUpdate")]
	pub on_update: Option<String>,
	#[serde(rename = "GRPCService")]
	pub grpc_service: Option<String>,
	#[serde(rename = "GRPCUseTLS")]
	pub grpc_use_tls: Option<bool>,
	#[serde(rename = "TaskName")]
	pub task_name: Option<String>,
	#[serde(rename = "SuccessBeforePassing")]
	pub success_before_passing: Option<i64>,
	#[serde(rename = "FailuresBeforeCritical")]
	pub failures_before_critical: Option<i64>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Connect {
	#[serde(rename = "Native")]
	pub native: Option<bool>,
	#[serde(rename = "Gateway")]
	pub gateway: Option<serde_json::Value>,
	#[serde(rename = "SidecarService")]
	pub sidecar_service: Option<SidecarService>,
	#[serde(rename = "SidecarTask")]
	pub sidecar_task: Option<Task>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SidecarService {
	#[serde(rename = "Tags")]
	pub tags: Option<Vec<String>>,
	#[serde(rename = "Port")]
	pub port: Option<String>,
	#[serde(rename = "Proxy")]
	pub proxy: Option<Proxy>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Proxy {
	#[serde(rename = "LocalServiceAddress")]
	pub local_service_address: Option<String>,
	#[serde(rename = "LocalServicePort")]
	pub local_service_port: Option<i64>,
	#[serde(rename = "ExposeConfig")]
	pub expose_config: Option<serde_json::Value>,
	#[serde(rename = "Upstreams")]
	pub upstreams: Option<Vec<Upstream>>,
	#[serde(rename = "Config")]
	pub config: Option<serde_json::Value>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Upstream {
	#[serde(rename = "DestinationName")]
	pub destination_name: Option<String>,
	#[serde(rename = "LocalBindPort")]
	pub local_bind_port: Option<i64>,
	#[serde(rename = "Datacenter")]
	pub datacenter: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Task {
	#[serde(rename = "Name")]
	pub name: Option<String>,
	#[serde(rename = "Driver")]
	pub driver: Option<String>,
	#[serde(rename = "User")]
	pub user: Option<String>,
	#[serde(rename = "Lifecycle")]
	pub lifecycle: Option<Lifecycle>,
	#[serde(rename = "Config")]
	pub config: Option<serde_json::Value>,
	#[serde(rename = "Constraints")]
	pub constraints: Option<serde_json::Value>,
	#[serde(rename = "Affinities")]
	pub affinities: Option<serde_json::Value>,
	#[serde(rename = "Env")]
	pub env: Option<HashMap<String, String>>,
	#[serde(rename = "Services")]
	pub services: Option<serde_json::Value>,
	#[serde(rename = "Resources")]
	pub resources: Option<Resources>,
	#[serde(rename = "RestartPolicy")]
	pub restart_policy: Option<RestartPolicy>,
	#[serde(rename = "Meta")]
	pub meta: Option<serde_json::Value>,
	#[serde(rename = "KillTimeout")]
	pub kill_timeout: Option<serde_json::Value>,
	#[serde(rename = "LogConfig")]
	pub log_config: Option<serde_json::Value>,
	#[serde(rename = "Artifacts")]
	pub artifacts: Option<serde_json::Value>,
	#[serde(rename = "Vault")]
	pub vault: Option<serde_json::Value>,
	#[serde(rename = "Templates")]
	pub templates: Option<serde_json::Value>,
	#[serde(rename = "DispatchPayload")]
	pub dispatch_payload: Option<serde_json::Value>,
	#[serde(rename = "VolumeMounts")]
	pub volume_mounts: Option<serde_json::Value>,
	#[serde(rename = "Leader")]
	pub leader: Option<bool>,
	#[serde(rename = "ShutdownDelay")]
	pub shutdown_delay: Option<i64>,
	#[serde(rename = "KillSignal")]
	pub kill_signal: Option<String>,
	#[serde(rename = "Kind")]
	pub kind: Option<String>,
	#[serde(rename = "ScalingPolicies")]
	pub scaling_policies: Option<serde_json::Value>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
	pub auth: Option<Vec<Auth>>,
	pub image: Option<String>,
	pub args: Option<Vec<String>>,
	pub command: Option<String>,
	pub network_mode: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Auth {
	pub password: Option<String>,
	pub server_address: Option<String>,
	pub username: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Lifecycle {
	#[serde(rename = "Hook")]
	pub hook: Option<String>,
	#[serde(rename = "Sidecar")]
	pub sidecar: Option<bool>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Resources {
	#[serde(rename = "CPU")]
	pub cpu: Option<i64>,
	#[serde(rename = "Cores")]
	pub cores: Option<i64>,
	#[serde(rename = "MemoryMB")]
	pub memory_mb: Option<i64>,
	#[serde(rename = "MemoryMaxMB")]
	pub memory_max_mb: Option<i64>,
	#[serde(rename = "DiskMB")]
	pub disk_mb: Option<serde_json::Value>,
	#[serde(rename = "Networks")]
	pub networks: Option<serde_json::Value>,
	#[serde(rename = "Devices")]
	pub devices: Option<serde_json::Value>,
	#[serde(rename = "IOPS")]
	pub iops: Option<serde_json::Value>,
}
