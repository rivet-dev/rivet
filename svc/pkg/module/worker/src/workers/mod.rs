mod create;
mod instance_create;
mod instance_destroy;
mod instance_version_set;
mod ns_version_set;
mod version_create;

chirp_worker::workers![
	create,
	instance_create,
	instance_destroy,
	instance_version_set,
	ns_version_set,
	version_create,
];
