// Copyright 2018 foundationdb-rs developers, https://github.com/Clikengo/foundationdb-rs/graphs/contributors
// Copyright 2013-2018 Apple, Inc and the FoundationDB project authors.
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

//! Error types for the Fdb crate

#![allow(non_upper_case_globals)]

use crate::options;
use crate::tuple::PackError;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};

#[allow(non_camel_case_types)]
pub type fdb_error_t = ::std::os::raw::c_int;
#[allow(non_camel_case_types)]
pub type fdb_bool_t = ::std::os::raw::c_int;

pub const success: i32 = 0;
pub const end_of_stream: i32 = 1;
pub const operation_failed: i32 = 1000;
pub const wrong_shard_server: i32 = 1001;
pub const operation_obsolete: i32 = 1002;
pub const cold_cache_server: i32 = 1003;
pub const timed_out: i32 = 1004;
pub const coordinated_state_conflict: i32 = 1005;
pub const all_alternatives_failed: i32 = 1006;
pub const transaction_too_old: i32 = 1007;
pub const no_more_servers: i32 = 1008;
pub const future_version: i32 = 1009;
pub const movekeys_conflict: i32 = 1010;
pub const tlog_stopped: i32 = 1011;
pub const server_request_queue_full: i32 = 1012;
pub const not_committed: i32 = 1020;
pub const commit_unknown_result: i32 = 1021;
pub const commit_unknown_result_fatal: i32 = 1022;
pub const transaction_cancelled: i32 = 1025;
pub const connection_failed: i32 = 1026;
pub const coordinators_changed: i32 = 1027;
pub const new_coordinators_timed_out: i32 = 1028;
pub const watch_cancelled: i32 = 1029;
pub const request_maybe_delivered: i32 = 1030;
pub const transaction_timed_out: i32 = 1031;
pub const too_many_watches: i32 = 1032;
pub const locality_information_unavailable: i32 = 1033;
pub const watches_disabled: i32 = 1034;
pub const default_error_or: i32 = 1035;
pub const accessed_unreadable: i32 = 1036;
pub const process_behind: i32 = 1037;
pub const database_locked: i32 = 1038;
pub const cluster_version_changed: i32 = 1039;
pub const external_client_already_loaded: i32 = 1040;
pub const lookup_failed: i32 = 1041;
pub const commit_proxy_memory_limit_exceeded: i32 = 1042;
pub const shutdown_in_progress: i32 = 1043;
pub const serialization_failed: i32 = 1044;
pub const connection_unreferenced: i32 = 1048;
pub const connection_idle: i32 = 1049;
pub const disk_adapter_reset: i32 = 1050;
pub const batch_transaction_throttled: i32 = 1051;
pub const dd_cancelled: i32 = 1052;
pub const dd_not_found: i32 = 1053;
pub const wrong_connection_file: i32 = 1054;
pub const version_already_compacted: i32 = 1055;
pub const local_config_changed: i32 = 1056;
pub const failed_to_reach_quorum: i32 = 1057;
pub const unsupported_format_version: i32 = 1058;
pub const unknown_change_feed: i32 = 1059;
pub const change_feed_not_registered: i32 = 1060;
pub const granule_assignment_conflict: i32 = 1061;
pub const change_feed_cancelled: i32 = 1062;
pub const blob_granule_file_load_error: i32 = 1063;
pub const blob_granule_transaction_too_old: i32 = 1064;
pub const blob_manager_replaced: i32 = 1065;
pub const change_feed_popped: i32 = 1066;
pub const remote_kvs_cancelled: i32 = 1067;
pub const page_header_wrong_page_id: i32 = 1068;
pub const page_header_checksum_failed: i32 = 1069;
pub const page_header_version_not_supported: i32 = 1070;
pub const page_encoding_not_supported: i32 = 1071;
pub const page_decoding_failed: i32 = 1072;
pub const unexpected_encoding_type: i32 = 1073;
pub const encryption_key_not_found: i32 = 1074;
pub const data_move_cancelled: i32 = 1075;
pub const data_move_dest_team_not_found: i32 = 1076;
pub const blob_worker_full: i32 = 1077;
pub const grv_proxy_memory_limit_exceeded: i32 = 1078;
pub const blob_granule_request_failed: i32 = 1079;
pub const storage_too_many_feed_streams: i32 = 1080;
pub const storage_engine_not_initialized: i32 = 1081;
pub const unknown_storage_engine: i32 = 1082;
pub const duplicate_snapshot_request: i32 = 1083;
pub const dd_config_changed: i32 = 1084;
pub const consistency_check_urgent_task_failed: i32 = 1085;
pub const data_move_conflict: i32 = 1086;
pub const consistency_check_urgent_duplicate_request: i32 = 1087;
pub const broken_promise: i32 = 1100;
pub const operation_cancelled: i32 = 1101;
pub const future_released: i32 = 1102;
pub const connection_leaked: i32 = 1103;
pub const never_reply: i32 = 1104;
pub const retry: i32 = 1105;
pub const recruitment_failed: i32 = 1200;
pub const move_to_removed_server: i32 = 1201;
pub const worker_removed: i32 = 1202;
pub const cluster_recovery_failed: i32 = 1203;
pub const master_max_versions_in_flight: i32 = 1204;
pub const tlog_failed: i32 = 1205;
pub const worker_recovery_failed: i32 = 1206;
pub const please_reboot: i32 = 1207;
pub const please_reboot_delete: i32 = 1208;
pub const commit_proxy_failed: i32 = 1209;
pub const resolver_failed: i32 = 1210;
pub const server_overloaded: i32 = 1211;
pub const backup_worker_failed: i32 = 1212;
pub const tag_throttled: i32 = 1213;
pub const grv_proxy_failed: i32 = 1214;
pub const dd_tracker_cancelled: i32 = 1215;
pub const failed_to_progress: i32 = 1216;
pub const invalid_cluster_id: i32 = 1217;
pub const restart_cluster_controller: i32 = 1218;
pub const please_reboot_kv_store: i32 = 1219;
pub const incompatible_software_version: i32 = 1220;
pub const audit_storage_failed: i32 = 1221;
pub const audit_storage_exceeded_request_limit: i32 = 1222;
pub const proxy_tag_throttled: i32 = 1223;
pub const key_value_store_deadline_exceeded: i32 = 1224;
pub const storage_quota_exceeded: i32 = 1225;
pub const audit_storage_error: i32 = 1226;
pub const master_failed: i32 = 1227;
pub const test_failed: i32 = 1228;
pub const retry_clean_up_datamove_tombstone_added: i32 = 1229;
pub const persist_new_audit_metadata_error: i32 = 1230;
pub const cancel_audit_storage_failed: i32 = 1231;
pub const audit_storage_cancelled: i32 = 1232;
pub const location_metadata_corruption: i32 = 1233;
pub const audit_storage_task_outdated: i32 = 1234;
pub const transaction_throttled_hot_shard: i32 = 1235;
pub const storage_replica_comparison_error: i32 = 1236;
pub const unreachable_storage_replica: i32 = 1237;
pub const bulkload_task_failed: i32 = 1238;
pub const bulkload_task_outdated: i32 = 1239;
pub const range_lock_failed: i32 = 1241;
pub const transaction_rejected_range_locked: i32 = 1242;
pub const bulkdump_task_failed: i32 = 1243;
pub const bulkdump_task_outdated: i32 = 1244;
pub const bulkload_fileset_invalid_filepath: i32 = 1245;
pub const bulkload_manifest_decode_error: i32 = 1246;
pub const range_lock_reject: i32 = 1247;
pub const range_unlock_reject: i32 = 1248;
pub const bulkload_dataset_not_cover_required_range: i32 = 1249;
pub const platform_error: i32 = 1500;
pub const large_alloc_failed: i32 = 1501;
pub const performance_counter_error: i32 = 1502;
pub const bad_allocator: i32 = 1503;
pub const io_error: i32 = 1510;
pub const file_not_found: i32 = 1511;
pub const bind_failed: i32 = 1512;
pub const file_not_readable: i32 = 1513;
pub const file_not_writable: i32 = 1514;
pub const no_cluster_file_found: i32 = 1515;
pub const file_too_large: i32 = 1516;
pub const non_sequential_op: i32 = 1517;
pub const http_bad_response: i32 = 1518;
pub const http_not_accepted: i32 = 1519;
pub const checksum_failed: i32 = 1520;
pub const io_timeout: i32 = 1521;
pub const file_corrupt: i32 = 1522;
pub const http_request_failed: i32 = 1523;
pub const http_auth_failed: i32 = 1524;
pub const http_bad_request_id: i32 = 1525;
pub const rest_invalid_uri: i32 = 1526;
pub const rest_invalid_rest_client_knob: i32 = 1527;
pub const rest_connectpool_key_not_found: i32 = 1528;
pub const lock_file_failure: i32 = 1529;
pub const rest_unsupported_protocol: i32 = 1530;
pub const rest_malformed_response: i32 = 1531;
pub const rest_max_base_cipher_len: i32 = 1532;
pub const resource_not_found: i32 = 1533;
pub const client_invalid_operation: i32 = 2000;
pub const commit_read_incomplete: i32 = 2002;
pub const test_specification_invalid: i32 = 2003;
pub const key_outside_legal_range: i32 = 2004;
pub const inverted_range: i32 = 2005;
pub const invalid_option_value: i32 = 2006;
pub const invalid_option: i32 = 2007;
pub const network_not_setup: i32 = 2008;
pub const network_already_setup: i32 = 2009;
pub const read_version_already_set: i32 = 2010;
pub const version_invalid: i32 = 2011;
pub const range_limits_invalid: i32 = 2012;
pub const invalid_database_name: i32 = 2013;
pub const attribute_not_found: i32 = 2014;
pub const future_not_set: i32 = 2015;
pub const future_not_error: i32 = 2016;
pub const used_during_commit: i32 = 2017;
pub const invalid_mutation_type: i32 = 2018;
pub const attribute_too_large: i32 = 2019;
pub const transaction_invalid_version: i32 = 2020;
pub const no_commit_version: i32 = 2021;
pub const environment_variable_network_option_failed: i32 = 2022;
pub const transaction_read_only: i32 = 2023;
pub const invalid_cache_eviction_policy: i32 = 2024;
pub const network_cannot_be_restarted: i32 = 2025;
pub const blocked_from_network_thread: i32 = 2026;
pub const invalid_config_db_range_read: i32 = 2027;
pub const invalid_config_db_key: i32 = 2028;
pub const invalid_config_path: i32 = 2029;
pub const mapper_bad_index: i32 = 2030;
pub const mapper_no_such_key: i32 = 2031;
pub const mapper_bad_range_decriptor: i32 = 2032;
pub const quick_get_key_values_has_more: i32 = 2033;
pub const quick_get_value_miss: i32 = 2034;
pub const quick_get_key_values_miss: i32 = 2035;
pub const blob_granule_no_ryw: i32 = 2036;
pub const blob_granule_not_materialized: i32 = 2037;
pub const get_mapped_key_values_has_more: i32 = 2038;
pub const get_mapped_range_reads_your_writes: i32 = 2039;
pub const checkpoint_not_found: i32 = 2040;
pub const key_not_tuple: i32 = 2041;
pub const value_not_tuple: i32 = 2042;
pub const mapper_not_tuple: i32 = 2043;
pub const invalid_checkpoint_format: i32 = 2044;
pub const invalid_throttle_quota_value: i32 = 2045;
pub const failed_to_create_checkpoint: i32 = 2046;
pub const failed_to_restore_checkpoint: i32 = 2047;
pub const failed_to_create_checkpoint_shard_metadata: i32 = 2048;
pub const address_parse_error: i32 = 2049;
pub const incompatible_protocol_version: i32 = 2100;
pub const transaction_too_large: i32 = 2101;
pub const key_too_large: i32 = 2102;
pub const value_too_large: i32 = 2103;
pub const connection_string_invalid: i32 = 2104;
pub const address_in_use: i32 = 2105;
pub const invalid_local_address: i32 = 2106;
pub const tls_error: i32 = 2107;
pub const unsupported_operation: i32 = 2108;
pub const too_many_tags: i32 = 2109;
pub const tag_too_long: i32 = 2110;
pub const too_many_tag_throttles: i32 = 2111;
pub const special_keys_cross_module_read: i32 = 2112;
pub const special_keys_no_module_found: i32 = 2113;
pub const special_keys_write_disabled: i32 = 2114;
pub const special_keys_no_write_module_found: i32 = 2115;
pub const special_keys_cross_module_clear: i32 = 2116;
pub const special_keys_api_failure: i32 = 2117;
pub const client_lib_invalid_metadata: i32 = 2118;
pub const client_lib_already_exists: i32 = 2119;
pub const client_lib_not_found: i32 = 2120;
pub const client_lib_not_available: i32 = 2121;
pub const client_lib_invalid_binary: i32 = 2122;
pub const no_external_client_provided: i32 = 2123;
pub const all_external_clients_failed: i32 = 2124;
pub const incompatible_client: i32 = 2125;
pub const tenant_name_required: i32 = 2130;
pub const tenant_not_found: i32 = 2131;
pub const tenant_already_exists: i32 = 2132;
pub const tenant_not_empty: i32 = 2133;
pub const invalid_tenant_name: i32 = 2134;
pub const tenant_prefix_allocator_conflict: i32 = 2135;
pub const tenants_disabled: i32 = 2136;
pub const illegal_tenant_access: i32 = 2138;
pub const invalid_tenant_group_name: i32 = 2139;
pub const invalid_tenant_configuration: i32 = 2140;
pub const cluster_no_capacity: i32 = 2141;
pub const tenant_removed: i32 = 2142;
pub const invalid_tenant_state: i32 = 2143;
pub const tenant_locked: i32 = 2144;
pub const invalid_cluster_name: i32 = 2160;
pub const invalid_metacluster_operation: i32 = 2161;
pub const cluster_already_exists: i32 = 2162;
pub const cluster_not_found: i32 = 2163;
pub const cluster_not_empty: i32 = 2164;
pub const cluster_already_registered: i32 = 2165;
pub const metacluster_no_capacity: i32 = 2166;
pub const management_cluster_invalid_access: i32 = 2167;
pub const tenant_creation_permanently_failed: i32 = 2168;
pub const cluster_removed: i32 = 2169;
pub const cluster_restoring: i32 = 2170;
pub const invalid_data_cluster: i32 = 2171;
pub const metacluster_mismatch: i32 = 2172;
pub const conflicting_restore: i32 = 2173;
pub const invalid_metacluster_configuration: i32 = 2174;
pub const unsupported_metacluster_version: i32 = 2175;
pub const api_version_unset: i32 = 2200;
pub const api_version_already_set: i32 = 2201;
pub const api_version_invalid: i32 = 2202;
pub const api_version_not_supported: i32 = 2203;
pub const api_function_missing: i32 = 2204;
pub const exact_mode_without_limits: i32 = 2210;
pub const invalid_tuple_data_type: i32 = 2250;
pub const invalid_tuple_index: i32 = 2251;
pub const key_not_in_subspace: i32 = 2252;
pub const manual_prefixes_not_enabled: i32 = 2253;
pub const prefix_in_partition: i32 = 2254;
pub const cannot_open_root_directory: i32 = 2255;
pub const directory_already_exists: i32 = 2256;
pub const directory_does_not_exist: i32 = 2257;
pub const parent_directory_does_not_exist: i32 = 2258;
pub const mismatched_layer: i32 = 2259;
pub const invalid_directory_layer_metadata: i32 = 2260;
pub const cannot_move_directory_between_partitions: i32 = 2261;
pub const cannot_use_partition_as_subspace: i32 = 2262;
pub const incompatible_directory_version: i32 = 2263;
pub const directory_prefix_not_empty: i32 = 2264;
pub const directory_prefix_in_use: i32 = 2265;
pub const invalid_destination_directory: i32 = 2266;
pub const cannot_modify_root_directory: i32 = 2267;
pub const invalid_uuid_size: i32 = 2268;
pub const invalid_versionstamp_size: i32 = 2269;
pub const backup_error: i32 = 2300;
pub const restore_error: i32 = 2301;
pub const backup_duplicate: i32 = 2311;
pub const backup_unneeded: i32 = 2312;
pub const backup_bad_block_size: i32 = 2313;
pub const backup_invalid_url: i32 = 2314;
pub const backup_invalid_info: i32 = 2315;
pub const backup_cannot_expire: i32 = 2316;
pub const backup_auth_missing: i32 = 2317;
pub const backup_auth_unreadable: i32 = 2318;
pub const backup_does_not_exist: i32 = 2319;
pub const backup_not_filterable_with_key_ranges: i32 = 2320;
pub const backup_not_overlapped_with_keys_filter: i32 = 2321;
pub const bucket_not_in_url: i32 = 2322;
pub const backup_parse_s3_response_failure: i32 = 2323;
pub const restore_invalid_version: i32 = 2361;
pub const restore_corrupted_data: i32 = 2362;
pub const restore_missing_data: i32 = 2363;
pub const restore_duplicate_tag: i32 = 2364;
pub const restore_unknown_tag: i32 = 2365;
pub const restore_unknown_file_type: i32 = 2366;
pub const restore_unsupported_file_version: i32 = 2367;
pub const restore_bad_read: i32 = 2368;
pub const restore_corrupted_data_padding: i32 = 2369;
pub const restore_destination_not_empty: i32 = 2370;
pub const restore_duplicate_uid: i32 = 2371;
pub const task_invalid_version: i32 = 2381;
pub const task_interrupted: i32 = 2382;
pub const invalid_encryption_key_file: i32 = 2383;
pub const blob_restore_missing_logs: i32 = 2384;
pub const blob_restore_corrupted_logs: i32 = 2385;
pub const blob_restore_invalid_manifest_url: i32 = 2386;
pub const blob_restore_corrupted_manifest: i32 = 2387;
pub const blob_restore_missing_manifest: i32 = 2388;
pub const blob_migrator_replaced: i32 = 2389;
pub const key_not_found: i32 = 2400;
pub const json_malformed: i32 = 2401;
pub const json_eof_expected: i32 = 2402;
pub const snap_disable_tlog_pop_failed: i32 = 2500;
pub const snap_storage_failed: i32 = 2501;
pub const snap_tlog_failed: i32 = 2502;
pub const snap_coord_failed: i32 = 2503;
pub const snap_enable_tlog_pop_failed: i32 = 2504;
pub const snap_path_not_whitelisted: i32 = 2505;
pub const snap_not_fully_recovered_unsupported: i32 = 2506;
pub const snap_log_anti_quorum_unsupported: i32 = 2507;
pub const snap_with_recovery_unsupported: i32 = 2508;
pub const snap_invalid_uid_string: i32 = 2509;
pub const encrypt_ops_error: i32 = 2700;
pub const encrypt_header_metadata_mismatch: i32 = 2701;
pub const encrypt_key_not_found: i32 = 2702;
pub const encrypt_key_ttl_expired: i32 = 2703;
pub const encrypt_header_authtoken_mismatch: i32 = 2704;
pub const encrypt_update_cipher: i32 = 2705;
pub const encrypt_invalid_id: i32 = 2706;
pub const encrypt_keys_fetch_failed: i32 = 2707;
pub const encrypt_invalid_kms_config: i32 = 2708;
pub const encrypt_unsupported: i32 = 2709;
pub const encrypt_mode_mismatch: i32 = 2710;
pub const encrypt_key_check_value_mismatch: i32 = 2711;
pub const encrypt_max_base_cipher_len: i32 = 2712;
pub const unknown_error: i32 = 4000;
pub const internal_error: i32 = 4100;
pub const not_implemented: i32 = 4200;
pub const permission_denied: i32 = 6000;
pub const unauthorized_attempt: i32 = 6001;
pub const digital_signature_ops_error: i32 = 6002;
pub const authorization_token_verify_failed: i32 = 6003;
pub const pkey_decode_error: i32 = 6004;
pub const pkey_encode_error: i32 = 6005;
pub const grpc_error: i32 = 7000;

pub fn fdb_get_error(code: fdb_error_t) -> &'static str {
	if code == success {
		"Success"
	} else if code == end_of_stream {
		"End of stream"
	} else if code == operation_failed {
		"Operation failed"
	} else if code == wrong_shard_server {
		"Shard is not available from this server"
	} else if code == operation_obsolete {
		"Operation result no longer necessary"
	} else if code == cold_cache_server {
		"Cache server is not warm for this range"
	} else if code == timed_out {
		"Operation timed out"
	} else if code == coordinated_state_conflict {
		"Conflict occurred while changing coordination information"
	} else if code == all_alternatives_failed {
		"All alternatives failed"
	} else if code == transaction_too_old {
		"Transaction is too old to perform reads or be committed"
	} else if code == no_more_servers {
		"Not enough physical servers available"
	} else if code == future_version {
		"Request for future version"
	} else if code == movekeys_conflict {
		"Conflicting attempts to change data distribution"
	} else if code == tlog_stopped {
		"TLog stopped"
	} else if code == server_request_queue_full {
		"Server request queue is full"
	} else if code == not_committed {
		"Transaction not committed due to conflict with another transaction"
	} else if code == commit_unknown_result {
		"Transaction may or may not have committed"
	} else if code == commit_unknown_result_fatal {
		"Idempotency id for transaction may have expired, so the commit status of the transaction cannot be determined"
	} else if code == transaction_cancelled {
		"Operation aborted because the transaction was cancelled"
	} else if code == connection_failed {
		"Network connection failed"
	} else if code == coordinators_changed {
		"Coordination servers have changed"
	} else if code == new_coordinators_timed_out {
		"New coordination servers did not respond in a timely way"
	} else if code == watch_cancelled {
		"Watch cancelled because storage server watch limit exceeded"
	} else if code == request_maybe_delivered {
		"Request may or may not have been delivered"
	} else if code == transaction_timed_out {
		"Operation aborted because the transaction timed out"
	} else if code == too_many_watches {
		"Too many watches currently set"
	} else if code == locality_information_unavailable {
		"Locality information not available"
	} else if code == watches_disabled {
		"Watches cannot be set if read your writes is disabled"
	} else if code == default_error_or {
		"Default error for an ErrorOr object"
	} else if code == accessed_unreadable {
		"Read or wrote an unreadable key"
	} else if code == process_behind {
		"Storage process does not have recent mutations"
	} else if code == database_locked {
		"Database is locked"
	} else if code == cluster_version_changed {
		"The protocol version of the cluster has changed"
	} else if code == external_client_already_loaded {
		"External client has already been loaded"
	} else if code == lookup_failed {
		"DNS lookup failed"
	} else if code == commit_proxy_memory_limit_exceeded {
		"CommitProxy commit memory limit exceeded"
	} else if code == shutdown_in_progress {
		"Operation no longer supported due to shutdown"
	} else if code == serialization_failed {
		"Failed to deserialize an object"
	} else if code == connection_unreferenced {
		"No peer references for connection"
	} else if code == connection_idle {
		"Connection closed after idle timeout"
	} else if code == disk_adapter_reset {
		"The disk queue adapter reset"
	} else if code == batch_transaction_throttled {
		"Batch GRV request rate limit exceeded"
	} else if code == dd_cancelled {
		"Data distribution components cancelled"
	} else if code == dd_not_found {
		"Data distributor not found"
	} else if code == wrong_connection_file {
		"Connection file mismatch"
	} else if code == version_already_compacted {
		"The requested changes have been compacted away"
	} else if code == local_config_changed {
		"Local configuration file has changed. Restart and apply these changes"
	} else if code == failed_to_reach_quorum {
		"Failed to reach quorum from configuration database nodes. Retry sending these requests"
	} else if code == unsupported_format_version {
		"Format version not supported"
	} else if code == unknown_change_feed {
		"Change feed not found"
	} else if code == change_feed_not_registered {
		"Change feed not registered"
	} else if code == granule_assignment_conflict {
		"Conflicting attempts to assign blob granules"
	} else if code == change_feed_cancelled {
		"Change feed was cancelled"
	} else if code == blob_granule_file_load_error {
		"Error loading a blob file during granule materialization"
	} else if code == blob_granule_transaction_too_old {
		"Read version is older than blob granule history supports"
	} else if code == blob_manager_replaced {
		"This blob manager has been replaced."
	} else if code == change_feed_popped {
		"Tried to read a version older than what has been popped from the change feed"
	} else if code == remote_kvs_cancelled {
		"The remote key-value store is cancelled"
	} else if code == page_header_wrong_page_id {
		"Page header does not match location on disk"
	} else if code == page_header_checksum_failed {
		"Page header checksum failed"
	} else if code == page_header_version_not_supported {
		"Page header version is not supported"
	} else if code == page_encoding_not_supported {
		"Page encoding type is not supported or not valid"
	} else if code == page_decoding_failed {
		"Page content decoding failed"
	} else if code == unexpected_encoding_type {
		"Page content decoding failed"
	} else if code == encryption_key_not_found {
		"Encryption key not found"
	} else if code == data_move_cancelled {
		"Data move was cancelled"
	} else if code == data_move_dest_team_not_found {
		"Dest team was not found for data move"
	} else if code == blob_worker_full {
		"Blob worker cannot take on more granule assignments"
	} else if code == grv_proxy_memory_limit_exceeded {
		"GetReadVersion proxy memory limit exceeded"
	} else if code == blob_granule_request_failed {
		"BlobGranule request failed"
	} else if code == storage_too_many_feed_streams {
		"Too many feed streams to a single storage server"
	} else if code == storage_engine_not_initialized {
		"Storage engine was never successfully initialized."
	} else if code == unknown_storage_engine {
		"Storage engine type is not recognized."
	} else if code == duplicate_snapshot_request {
		"A duplicate snapshot request has been sent, the old request is discarded."
	} else if code == dd_config_changed {
		"DataDistribution configuration changed."
	} else if code == consistency_check_urgent_task_failed {
		"Consistency check urgent task is failed"
	} else if code == data_move_conflict {
		"Data move conflict in SS"
	} else if code == consistency_check_urgent_duplicate_request {
		"Consistency check urgent got a duplicate request"
	} else if code == broken_promise {
		"Broken promise"
	} else if code == operation_cancelled {
		"Asynchronous operation cancelled"
	} else if code == future_released {
		"Future has been released"
	} else if code == connection_leaked {
		"Connection object leaked"
	} else if code == never_reply {
		"Never reply to the request"
	} else if code == retry {
		"Retry operation"
	}
	// Be careful, catching this will delete the data of a storage server or tlog permanently
	else if code == recruitment_failed {
		"Recruitment of a server failed"
	} else if code == move_to_removed_server {
		"Attempt to move keys to a storage server that was removed"
	}
	// Be careful, catching this will delete the data of a storage server or tlog permanently
	else if code == worker_removed {
		"Normal worker shut down"
	} else if code == cluster_recovery_failed {
		"Cluster recovery failed"
	} else if code == master_max_versions_in_flight {
		"Master hit maximum number of versions in flight"
	}
	// similar to tlog_stopped, but the tlog has actually died
	else if code == tlog_failed {
		"Cluster recovery terminating because a TLog failed"
	} else if code == worker_recovery_failed {
		"Recovery of a worker process failed"
	} else if code == please_reboot {
		"Reboot of server process requested"
	} else if code == please_reboot_delete {
		"Reboot of server process requested, with deletion of state"
	} else if code == commit_proxy_failed {
		"Master terminating because a CommitProxy failed"
	} else if code == resolver_failed {
		"Cluster recovery terminating because a Resolver failed"
	} else if code == server_overloaded {
		"Server is under too much load and cannot respond"
	} else if code == backup_worker_failed {
		"Cluster recovery terminating because a backup worker failed"
	} else if code == tag_throttled {
		"Transaction tag is being throttled"
	} else if code == grv_proxy_failed {
		"Cluster recovery terminating because a GRVProxy failed"
	} else if code == dd_tracker_cancelled {
		"The data distribution tracker has been cancelled"
	} else if code == failed_to_progress {
		"Process has failed to make sufficient progress"
	} else if code == invalid_cluster_id {
		"Attempted to join cluster with a different cluster ID"
	} else if code == restart_cluster_controller {
		"Restart cluster controller process"
	} else if code == please_reboot_kv_store {
		"Need to reboot the storage engine"
	} else if code == incompatible_software_version {
		"Current software does not support database format"
	} else if code == audit_storage_failed {
		"Validate storage consistency operation failed"
	} else if code == audit_storage_exceeded_request_limit {
		"Exceeded the max number of allowed concurrent audit storage requests"
	} else if code == proxy_tag_throttled {
		"Exceeded maximum proxy tag throttling duration"
	} else if code == key_value_store_deadline_exceeded {
		"Exceeded maximum time allowed to read or write."
	} else if code == storage_quota_exceeded {
		"Exceeded the maximum storage quota allocated to the tenant."
	} else if code == audit_storage_error {
		"Found data corruption"
	} else if code == master_failed {
		"Cluster recovery terminating because master has failed"
	} else if code == test_failed {
		"Test failed"
	} else if code == retry_clean_up_datamove_tombstone_added {
		"Need background datamove cleanup"
	} else if code == persist_new_audit_metadata_error {
		"Persist new audit metadata error"
	} else if code == cancel_audit_storage_failed {
		"Failed to cancel an audit"
	} else if code == audit_storage_cancelled {
		"Audit has been cancelled"
	} else if code == location_metadata_corruption {
		"Found location metadata corruption"
	} else if code == audit_storage_task_outdated {
		"Audit task is scheduled by an outdated DD"
	} else if code == transaction_throttled_hot_shard {
		"Transaction throttled due to hot shard"
	} else if code == storage_replica_comparison_error {
		"Storage replicas not consistent"
	} else if code == unreachable_storage_replica {
		"Storage replica cannot be reached"
	} else if code == bulkload_task_failed {
		"Bulk loading task failed"
	} else if code == bulkload_task_outdated {
		"Bulk loading task outdated"
	} else if code == range_lock_failed {
		"Lock range failed"
	} else if code == transaction_rejected_range_locked {
		"Transaction rejected due to range lock"
	} else if code == bulkdump_task_failed {
		"Bulk dumping task failed"
	} else if code == bulkdump_task_outdated {
		"Bulk dumping task outdated"
	} else if code == bulkload_fileset_invalid_filepath {
		"Bulkload fileset provides invalid filepath"
	} else if code == bulkload_manifest_decode_error {
		"Bulkload manifest string is failed to decode"
	} else if code == range_lock_reject {
		"Range lock is rejected"
	} else if code == range_unlock_reject {
		"Range unlock is rejected"
	} else if code == bulkload_dataset_not_cover_required_range {
		"Bulkload dataset does not cover the required range"
	}
	// 15xx Platform errors
	else if code == platform_error {
		"Platform error"
	} else if code == large_alloc_failed {
		"Large block allocation failed"
	} else if code == performance_counter_error {
		"QueryPerformanceCounter error"
	} else if code == bad_allocator {
		"Null allocator was used to allocate memory"
	} else if code == io_error {
		"Disk i/o operation failed"
	} else if code == file_not_found {
		"File not found"
	} else if code == bind_failed {
		"Unable to bind to network"
	} else if code == file_not_readable {
		"File could not be read"
	} else if code == file_not_writable {
		"File could not be written"
	} else if code == no_cluster_file_found {
		"No cluster file found in current directory or default location"
	} else if code == file_too_large {
		"File too large to be read"
	} else if code == non_sequential_op {
		"Non sequential file operation not allowed"
	} else if code == http_bad_response {
		"HTTP response was badly formed"
	} else if code == http_not_accepted {
		"HTTP request not accepted"
	} else if code == checksum_failed {
		"A data checksum failed"
	} else if code == io_timeout {
		"A disk IO operation failed to complete in a timely manner"
	} else if code == file_corrupt {
		"A structurally corrupt data file was detected"
	} else if code == http_request_failed {
		"HTTP response code not received or indicated failure"
	} else if code == http_auth_failed {
		"HTTP request failed due to bad credentials"
	} else if code == http_bad_request_id {
		"HTTP response contained an unexpected X-Request-ID header"
	} else if code == rest_invalid_uri {
		"Invalid REST URI"
	} else if code == rest_invalid_rest_client_knob {
		"Invalid RESTClient knob"
	} else if code == rest_connectpool_key_not_found {
		"ConnectKey not found in connection pool"
	} else if code == lock_file_failure {
		"Unable to lock the file"
	} else if code == rest_unsupported_protocol {
		"Unsupported REST protocol"
	} else if code == rest_malformed_response {
		"Malformed REST response"
	} else if code == rest_max_base_cipher_len {
		"Max BaseCipher length violation"
	} else if code == resource_not_found {
		"Requested resource was not found"
	}
	// 2xxx Attempt (presumably by a _client_) to do something illegal. If an error is known to
	// be internally caused, it should be 41xx
	else if code == client_invalid_operation {
		"Invalid API call"
	} else if code == commit_read_incomplete {
		"Commit with incomplete read"
	} else if code == test_specification_invalid {
		"Invalid test specification"
	} else if code == key_outside_legal_range {
		"Key outside legal range"
	} else if code == inverted_range {
		"Range begin key larger than end key"
	} else if code == invalid_option_value {
		"Option set with an invalid value"
	} else if code == invalid_option {
		"Option not valid in this context"
	} else if code == network_not_setup {
		"Action not possible before the network is configured"
	} else if code == network_already_setup {
		"Network can be configured only once"
	} else if code == read_version_already_set {
		"Transaction already has a read version set"
	} else if code == version_invalid {
		"Version not valid"
	} else if code == range_limits_invalid {
		"Range limits not valid"
	} else if code == invalid_database_name {
		"Database name must be 'DB'"
	} else if code == attribute_not_found {
		"Attribute not found"
	} else if code == future_not_set {
		"Future not ready"
	} else if code == future_not_error {
		"Future not an error"
	} else if code == used_during_commit {
		"Operation issued while a commit was outstanding"
	} else if code == invalid_mutation_type {
		"Unrecognized atomic mutation type"
	} else if code == attribute_too_large {
		"Attribute too large for type int"
	} else if code == transaction_invalid_version {
		"Transaction does not have a valid commit version"
	} else if code == no_commit_version {
		"Transaction is read-only and therefore does not have a commit version"
	} else if code == environment_variable_network_option_failed {
		"Environment variable network option could not be set"
	} else if code == transaction_read_only {
		"Attempted to commit a transaction specified as read-only"
	} else if code == invalid_cache_eviction_policy {
		"Invalid cache eviction policy, only random and lru are supported"
	} else if code == network_cannot_be_restarted {
		"Network can only be started once"
	} else if code == blocked_from_network_thread {
		"Detected a deadlock in a callback called from the network thread"
	} else if code == invalid_config_db_range_read {
		"Invalid configuration database range read"
	} else if code == invalid_config_db_key {
		"Invalid configuration database key provided"
	} else if code == invalid_config_path {
		"Invalid configuration path"
	} else if code == mapper_bad_index {
		"The index in K[] or V[] is not a valid number or out of range"
	} else if code == mapper_no_such_key {
		"A mapped key is not set in database"
	} else if code == mapper_bad_range_decriptor {
		"\"{...}\" must be the last element of the mapper tuple"
	} else if code == quick_get_key_values_has_more {
		"One of the mapped range queries is too large"
	} else if code == quick_get_value_miss {
		"Found a mapped key that is not served in the same SS"
	} else if code == quick_get_key_values_miss {
		"Found a mapped range that is not served in the same SS"
	} else if code == blob_granule_no_ryw {
		"Blob Granule Read Transactions must be specified as ryw-disabled"
	} else if code == blob_granule_not_materialized {
		"Blob Granule Read was not materialized"
	} else if code == get_mapped_key_values_has_more {
		"getMappedRange does not support continuation for now"
	} else if code == get_mapped_range_reads_your_writes {
		"getMappedRange tries to read data that were previously written in the transaction"
	} else if code == checkpoint_not_found {
		"Checkpoint not found"
	} else if code == key_not_tuple {
		"The key cannot be parsed as a tuple"
	} else if code == value_not_tuple {
		"The value cannot be parsed as a tuple"
	} else if code == mapper_not_tuple {
		"The mapper cannot be parsed as a tuple"
	} else if code == invalid_checkpoint_format {
		"Invalid checkpoint format"
	} else if code == invalid_throttle_quota_value {
		"Invalid quota value. Note that reserved_throughput cannot exceed total_throughput"
	} else if code == failed_to_create_checkpoint {
		"Failed to create a checkpoint"
	} else if code == failed_to_restore_checkpoint {
		"Failed to restore a checkpoint"
	} else if code == failed_to_create_checkpoint_shard_metadata {
		"Failed to dump shard metadata for a checkpoint to a sst file"
	} else if code == address_parse_error {
		"Failed to parse address"
	} else if code == incompatible_protocol_version {
		"Incompatible protocol version"
	} else if code == transaction_too_large {
		"Transaction exceeds byte limit"
	} else if code == key_too_large {
		"Key length exceeds limit"
	} else if code == value_too_large {
		"Value length exceeds limit"
	} else if code == connection_string_invalid {
		"Connection string invalid"
	} else if code == address_in_use {
		"Local address in use"
	} else if code == invalid_local_address {
		"Invalid local address"
	} else if code == tls_error {
		"TLS error"
	} else if code == unsupported_operation {
		"Operation is not supported"
	} else if code == too_many_tags {
		"Too many tags set on transaction"
	} else if code == tag_too_long {
		"Tag set on transaction is too long"
	} else if code == too_many_tag_throttles {
		"Too many tag throttles have been created"
	} else if code == special_keys_cross_module_read {
		"Special key space range read crosses modules. Refer to the `special_key_space_relaxed' transaction option for more details."
	} else if code == special_keys_no_module_found {
		"Special key space range read does not intersect a module. Refer to the `special_key_space_relaxed' transaction option for more details."
	} else if code == special_keys_write_disabled {
		"Special Key space is not allowed to write by default. Refer to the `special_key_space_enable_writes` transaction option for more details."
	} else if code == special_keys_no_write_module_found {
		"Special key space key or keyrange in set or clear does not intersect a module"
	} else if code == special_keys_cross_module_clear {
		"Special key space clear crosses modules"
	} else if code == special_keys_api_failure {
		"Api call through special keys failed. For more information, call get on special key 0xff0xff/error_message to get a json string of the error message."
	} else if code == client_lib_invalid_metadata {
		"Invalid client library metadata."
	} else if code == client_lib_already_exists {
		"Client library with same identifier already exists on the cluster."
	} else if code == client_lib_not_found {
		"Client library for the given identifier not found."
	} else if code == client_lib_not_available {
		"Client library exists, but is not available for download."
	} else if code == client_lib_invalid_binary {
		"Invalid client library binary."
	} else if code == no_external_client_provided {
		"No external client library provided."
	} else if code == all_external_clients_failed {
		"All external clients have failed."
	} else if code == incompatible_client {
		"None of the available clients match the protocol version of the cluster."
	} else if code == tenant_name_required {
		"Tenant name must be specified to access data in the cluster"
	} else if code == tenant_not_found {
		"Tenant does not exist"
	} else if code == tenant_already_exists {
		"A tenant with the given name already exists"
	} else if code == tenant_not_empty {
		"Cannot delete a non-empty tenant"
	} else if code == invalid_tenant_name {
		"Tenant name cannot begin with \\xff"
	} else if code == tenant_prefix_allocator_conflict {
		"The database already has keys stored at the prefix allocated for the tenant"
	} else if code == tenants_disabled {
		"Tenants have been disabled in the cluster"
	} else if code == illegal_tenant_access {
		"Illegal tenant access"
	} else if code == invalid_tenant_group_name {
		"Tenant group name cannot begin with \\xff"
	} else if code == invalid_tenant_configuration {
		"Tenant configuration is invalid"
	} else if code == cluster_no_capacity {
		"Cluster does not have capacity to perform the specified operation"
	} else if code == tenant_removed {
		"The tenant was removed"
	} else if code == invalid_tenant_state {
		"Operation cannot be applied to tenant in its current state"
	} else if code == tenant_locked {
		"Tenant is locked"
	} else if code == invalid_cluster_name {
		"Data cluster name cannot begin with \\xff"
	} else if code == invalid_metacluster_operation {
		"Metacluster operation performed on non-metacluster"
	} else if code == cluster_already_exists {
		"A data cluster with the given name already exists"
	} else if code == cluster_not_found {
		"Data cluster does not exist"
	} else if code == cluster_not_empty {
		"Cluster must be empty"
	} else if code == cluster_already_registered {
		"Data cluster is already registered with a metacluster"
	} else if code == metacluster_no_capacity {
		"Metacluster does not have capacity to create new tenants"
	} else if code == management_cluster_invalid_access {
		"Standard transactions cannot be run against the management cluster"
	} else if code == tenant_creation_permanently_failed {
		"The tenant creation did not complete in a timely manner and has permanently failed"
	} else if code == cluster_removed {
		"The cluster is being removed from the metacluster"
	} else if code == cluster_restoring {
		"The cluster is being restored to the metacluster"
	} else if code == invalid_data_cluster {
		"The data cluster being restored has no record of its metacluster"
	} else if code == metacluster_mismatch {
		"The cluster does not have the expected name or is associated with a different metacluster"
	} else if code == conflicting_restore {
		"Another restore is running for the same data cluster"
	} else if code == invalid_metacluster_configuration {
		"Metacluster configuration is invalid"
	} else if code == unsupported_metacluster_version {
		"Client is not compatible with the metacluster"
	}
	// 2200 - errors from bindings and official APIs
	else if code == api_version_unset {
		"API version is not set"
	} else if code == api_version_already_set {
		"API version may be set only once"
	} else if code == api_version_invalid {
		"API version not valid"
	} else if code == api_version_not_supported {
		"API version not supported"
	} else if code == api_function_missing {
		"Failed to load a required FDB API function."
	} else if code == exact_mode_without_limits {
		"EXACT streaming mode requires limits, but none were given"
	} else if code == invalid_tuple_data_type {
		"Unrecognized data type in packed tuple"
	} else if code == invalid_tuple_index {
		"Tuple does not have element at specified index"
	} else if code == key_not_in_subspace {
		"Cannot unpack key that is not in subspace"
	} else if code == manual_prefixes_not_enabled {
		"Cannot specify a prefix unless manual prefixes are enabled"
	} else if code == prefix_in_partition {
		"Cannot specify a prefix in a partition"
	} else if code == cannot_open_root_directory {
		"Root directory cannot be opened"
	} else if code == directory_already_exists {
		"Directory already exists"
	} else if code == directory_does_not_exist {
		"Directory does not exist"
	} else if code == parent_directory_does_not_exist {
		"Directory's parent does not exist"
	} else if code == mismatched_layer {
		"Directory has already been created with a different layer string"
	} else if code == invalid_directory_layer_metadata {
		"Invalid directory layer metadata"
	} else if code == cannot_move_directory_between_partitions {
		"Directory cannot be moved between partitions"
	} else if code == cannot_use_partition_as_subspace {
		"Directory partition cannot be used as subspace"
	} else if code == incompatible_directory_version {
		"Directory layer was created with an incompatible version"
	} else if code == directory_prefix_not_empty {
		"Database has keys stored at the prefix chosen by the automatic prefix allocator"
	} else if code == directory_prefix_in_use {
		"Directory layer already has a conflicting prefix"
	} else if code == invalid_destination_directory {
		"Target directory is invalid"
	} else if code == cannot_modify_root_directory {
		"Root directory cannot be modified"
	} else if code == invalid_uuid_size {
		"UUID is not sixteen bytes"
	} else if code == invalid_versionstamp_size {
		"Versionstamp is not exactly twelve bytes"
	}
	// 2300 - backup and restore errors
	else if code == backup_error {
		"Backup error"
	} else if code == restore_error {
		"Restore error"
	} else if code == backup_duplicate {
		"Backup duplicate request"
	} else if code == backup_unneeded {
		"Backup unneeded request"
	} else if code == backup_bad_block_size {
		"Backup file block size too small"
	} else if code == backup_invalid_url {
		"Backup Container URL invalid"
	} else if code == backup_invalid_info {
		"Backup Container info invalid"
	} else if code == backup_cannot_expire {
		"Cannot expire requested data from backup without violating minimum restorability"
	} else if code == backup_auth_missing {
		"Cannot find authentication details (such as a password or secret key) for the specified Backup Container URL"
	} else if code == backup_auth_unreadable {
		"Cannot read or parse one or more sources of authentication information for Backup Container URLs"
	} else if code == backup_does_not_exist {
		"Backup does not exist"
	} else if code == backup_not_filterable_with_key_ranges {
		"Backup before 6.3 cannot be filtered with key ranges"
	} else if code == backup_not_overlapped_with_keys_filter {
		"Backup key ranges doesn't overlap with key ranges filter"
	} else if code == bucket_not_in_url {
		"bucket is not in the URL for backup"
	} else if code == backup_parse_s3_response_failure {
		"cannot parse s3 response properly"
	} else if code == restore_invalid_version {
		"Invalid restore version"
	} else if code == restore_corrupted_data {
		"Corrupted backup data"
	} else if code == restore_missing_data {
		"Missing backup data"
	} else if code == restore_duplicate_tag {
		"Restore duplicate request"
	} else if code == restore_unknown_tag {
		"Restore tag does not exist"
	} else if code == restore_unknown_file_type {
		"Unknown backup/restore file type"
	} else if code == restore_unsupported_file_version {
		"Unsupported backup file version"
	} else if code == restore_bad_read {
		"Unexpected number of bytes read"
	} else if code == restore_corrupted_data_padding {
		"Backup file has unexpected padding bytes"
	} else if code == restore_destination_not_empty {
		"Attempted to restore into a non-empty destination database"
	} else if code == restore_duplicate_uid {
		"Attempted to restore using a UID that had been used for an aborted restore"
	} else if code == task_invalid_version {
		"Invalid task version"
	} else if code == task_interrupted {
		"Task execution stopped due to timeout, abort, or completion by another worker"
	} else if code == invalid_encryption_key_file {
		"The provided encryption key file has invalid contents"
	} else if code == blob_restore_missing_logs {
		"Missing mutation logs"
	} else if code == blob_restore_corrupted_logs {
		"Corrupted mutation logs"
	} else if code == blob_restore_invalid_manifest_url {
		"Invalid manifest URL"
	} else if code == blob_restore_corrupted_manifest {
		"Corrupted manifest"
	} else if code == blob_restore_missing_manifest {
		"Missing manifest"
	} else if code == blob_migrator_replaced {
		"Blob migrator is replaced"
	} else if code == key_not_found {
		"Expected key is missing"
	} else if code == json_malformed {
		"JSON string was malformed"
	} else if code == json_eof_expected {
		"JSON string did not terminate where expected"
	}
	// 2500 - disk snapshot based backup errors
	else if code == snap_disable_tlog_pop_failed {
		"Failed to disable tlog pops"
	} else if code == snap_storage_failed {
		"Failed to snapshot storage nodes"
	} else if code == snap_tlog_failed {
		"Failed to snapshot TLog nodes"
	} else if code == snap_coord_failed {
		"Failed to snapshot coordinator nodes"
	} else if code == snap_enable_tlog_pop_failed {
		"Failed to enable tlog pops"
	} else if code == snap_path_not_whitelisted {
		"Snapshot create binary path not whitelisted"
	} else if code == snap_not_fully_recovered_unsupported {
		"Unsupported when the cluster is not fully recovered"
	} else if code == snap_log_anti_quorum_unsupported {
		"Unsupported when log anti quorum is configured"
	} else if code == snap_with_recovery_unsupported {
		"Cluster recovery during snapshot operation not supported"
	} else if code == snap_invalid_uid_string {
		"The given uid string is not a 32-length hex string"
	}
	// 27XX - Encryption operations errors
	else if code == encrypt_ops_error {
		"Encryption operation error"
	} else if code == encrypt_header_metadata_mismatch {
		"Encryption header metadata mismatch"
	} else if code == encrypt_key_not_found {
		"Expected encryption key is missing"
	} else if code == encrypt_key_ttl_expired {
		"Expected encryption key TTL has expired"
	} else if code == encrypt_header_authtoken_mismatch {
		"Encryption header authentication token mismatch"
	} else if code == encrypt_update_cipher {
		"Attempt to update encryption cipher key"
	} else if code == encrypt_invalid_id {
		"Invalid encryption cipher details"
	} else if code == encrypt_keys_fetch_failed {
		"Encryption keys fetch from external KMS failed"
	} else if code == encrypt_invalid_kms_config {
		"Invalid encryption/kms configuration: discovery-url, validation-token, endpoint etc."
	} else if code == encrypt_unsupported {
		"Encryption not supported"
	} else if code == encrypt_mode_mismatch {
		"Encryption mode mismatch with configuration"
	} else if code == encrypt_key_check_value_mismatch {
		"Encryption key-check-value mismatch"
	} else if code == encrypt_max_base_cipher_len {
		"Max BaseCipher buffer length violation"
	}
	// 4xxx Internal errors (those that should be generated only by bugs) are decimal 4xxx
	// C++ exception not of type Error
	else if code == unknown_error {
		"An unknown error occurred"
	} else if code == internal_error {
		"An internal error occurred"
	} else if code == not_implemented {
		"Not implemented yet"
	}
	// 6xxx Authorization and authentication error codes
	else if code == permission_denied {
		"Client tried to access unauthorized data"
	} else if code == unauthorized_attempt {
		"A untrusted client tried to send a message to a private endpoint"
	} else if code == digital_signature_ops_error {
		"Digital signature operation error"
	} else if code == authorization_token_verify_failed {
		"Failed to verify authorization token"
	} else if code == pkey_decode_error {
		"Failed to decode public/private key"
	} else if code == pkey_encode_error {
		"Failed to encode public/private key"
	}
	// gRPC error
	else if code == grpc_error {
		"gRPC Error"
	} else {
		"Unknown error"
	}
}

pub fn fdb_error_predicate(predicate_test: options::ErrorPredicate, code: fdb_error_t) -> bool {
	if predicate_test == options::ErrorPredicate::Retryable {
		return fdb_error_predicate(options::ErrorPredicate::MaybeCommitted, code)
			|| fdb_error_predicate(options::ErrorPredicate::RetryableNotCommitted, code);
	}
	if predicate_test == options::ErrorPredicate::MaybeCommitted {
		return code == commit_unknown_result || code == cluster_version_changed;
	}
	if predicate_test == options::ErrorPredicate::RetryableNotCommitted {
		return code == not_committed
			|| code == transaction_too_old
			|| code == future_version
			|| code == database_locked
			|| code == grv_proxy_memory_limit_exceeded
			|| code == commit_proxy_memory_limit_exceeded
			|| code == transaction_throttled_hot_shard
			|| code == batch_transaction_throttled
			|| code == process_behind
			|| code == tag_throttled
			|| code == proxy_tag_throttled
			|| code == transaction_rejected_range_locked;
	}

	false
}

/// Error returned when attempting to access metrics on a transaction that wasn't created with metrics instrumentation.
///
/// This error occurs when calling methods like `set_custom_metric` or `increment_custom_metric` on a
/// transaction that was created without metrics instrumentation (i.e., using `create_trx` instead of
/// `create_instrumented_trx`).
#[derive(Debug)]
pub struct TransactionMetricsNotFound;

impl std::fmt::Display for TransactionMetricsNotFound {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Transaction metrics not found")
	}
}

impl std::error::Error for TransactionMetricsNotFound {}

/// The Standard Error type of FoundationDB
#[derive(Debug, Copy, Clone)]
pub struct FdbError {
	/// The FoundationDB error code
	error_code: i32,
}

impl FdbError {
	/// Converts from a raw foundationDB error code
	pub fn from_code(error_code: fdb_error_t) -> Self {
		Self { error_code }
	}

	pub fn message(self) -> &'static str {
		fdb_get_error(self.error_code)
	}

	fn is_error_predicate(self, predicate: options::ErrorPredicate) -> bool {
		fdb_error_predicate(predicate, self.error_code)
	}

	/// Indicates the transaction may have succeeded, though not in a way the system can verify.
	pub fn is_maybe_committed(self) -> bool {
		self.is_error_predicate(options::ErrorPredicate::MaybeCommitted)
	}

	/// Indicates the operations in the transactions should be retried because of transient error.
	pub fn is_retryable(self) -> bool {
		self.is_error_predicate(options::ErrorPredicate::Retryable)
	}

	/// Indicates the transaction has not committed, though in a way that can be retried.
	pub fn is_retryable_not_committed(self) -> bool {
		self.is_error_predicate(options::ErrorPredicate::RetryableNotCommitted)
	}

	/// Raw foundationdb error code
	pub fn code(self) -> i32 {
		self.error_code
	}
}

impl fmt::Display for FdbError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		std::fmt::Display::fmt(&self.message(), f)
	}
}

impl std::error::Error for FdbError {}

/// Alias for `Result<..., FdbError>`
pub type FdbResult<T = ()> = Result<T, FdbError>;

/// This error represent all errors that can be throwed by `db.run`.
/// Layer developers may use the `CustomError`.
pub enum FdbBindingError {
	NonRetryableFdbError(FdbError),
	PackError(PackError),
	/// A reference to the `RetryableTransaction` has been kept
	ReferenceToTransactionKept,
	/// A custom error that layer developers can use
	CustomError(Box<dyn std::error::Error + Send + Sync>),
	/// Error returned when attempting to access metrics on a transaction that wasn't created with metrics instrumentation
	TransactionMetricsNotFound,
}

impl FdbBindingError {
	/// Returns the underlying `FdbError`, if any.
	pub fn get_fdb_error(&self) -> Option<FdbError> {
		match *self {
			Self::NonRetryableFdbError(error) => Some(error),
			Self::CustomError(ref error) => {
				if let Some(e) = error.downcast_ref::<FdbError>() {
					Some(*e)
				} else if let Some(e) = error.downcast_ref::<FdbBindingError>() {
					e.get_fdb_error()
				} else {
					None
				}
			}
			_ => None,
		}
	}
}

impl From<FdbError> for FdbBindingError {
	fn from(e: FdbError) -> Self {
		Self::NonRetryableFdbError(e)
	}
}

impl From<TransactionMetricsNotFound> for FdbBindingError {
	fn from(_e: TransactionMetricsNotFound) -> Self {
		Self::TransactionMetricsNotFound
	}
}

impl FdbBindingError {
	/// create a new custom error
	pub fn new_custom_error(e: Box<dyn std::error::Error + Send + Sync>) -> Self {
		Self::CustomError(e)
	}
}

impl Debug for FdbBindingError {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self {
			FdbBindingError::NonRetryableFdbError(err) => write!(f, "{err:?}"),
			FdbBindingError::PackError(err) => write!(f, "{err:?}"),
			FdbBindingError::ReferenceToTransactionKept => {
				write!(f, "Reference to transaction kept")
			}
			FdbBindingError::CustomError(err) => write!(f, "{err:?}"),
			FdbBindingError::TransactionMetricsNotFound => {
				write!(f, "Transaction metrics not found")
			}
		}
	}
}

impl Display for FdbBindingError {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		std::fmt::Debug::fmt(&self, f)
	}
}

impl std::error::Error for FdbBindingError {}
