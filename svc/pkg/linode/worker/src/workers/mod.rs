pub mod prebake_install_complete;
pub mod prebake_provision;

chirp_worker::workers![prebake_install_complete, prebake_provision,];
