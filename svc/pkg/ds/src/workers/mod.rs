mod drain_all;
mod webhook;

chirp_worker::workers![drain_all,];
