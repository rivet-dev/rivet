pub mod create;
pub mod datacenter_create;
pub mod datacenter_scale;
pub mod datacenter_tls_issue;
pub mod datacenter_update;
pub mod game_link;
pub mod nomad_node_drain_complete;
pub mod nomad_node_registered;
pub mod server_destroy;
pub mod server_dns_create;
pub mod server_dns_delete;
pub mod server_drain;
pub mod server_install;
pub mod server_install_complete;
pub mod server_provision;
pub mod server_taint;
pub mod server_undrain;

chirp_worker::workers![
	server_taint,
	create,
	datacenter_create,
	datacenter_scale,
	datacenter_tls_issue,
	datacenter_update,
	game_link,
	nomad_node_drain_complete,
	nomad_node_registered,
	server_destroy,
	server_dns_create,
	server_dns_delete,
	server_drain,
	server_install_complete,
	server_install,
	server_provision,
	server_undrain,
];
