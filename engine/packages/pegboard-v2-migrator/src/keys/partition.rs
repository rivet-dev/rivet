//! The migrator's one volume. It lives on V3's own log quorum, so its state is as durable as any
//! V3 write, and every write to it is fenced by the singleton tenure (see `V2MigratorFenceKey`).

rivet_keys::volume_key! { pub V2MigratorVolume() => [V2_MIGRATOR]; }
