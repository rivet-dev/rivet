mod drain_all;
mod undrain_all;
mod webhook;

chirp_worker::workers![drain_all, undrain_all];
