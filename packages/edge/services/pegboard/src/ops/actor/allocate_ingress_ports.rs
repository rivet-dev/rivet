use chirp_workflow::prelude::*;
use fdb_util::{end_of_key_range, FormalKey, SNAPSHOT};
use foundationdb::{
	self as fdb,
	options::{ConflictRangeType, StreamingMode},
};
use futures_util::TryStreamExt;
use rand::Rng;

use crate::{keys, types::GameGuardProtocol};

/// Allocates X ingress ports per given protocol uniquely in the global FDB index.
#[derive(Debug, Default)]
pub(crate) struct Input {
	pub actor_id: Uuid,
	/// How many ports of each protocol are needed.
	pub ports: Vec<(GameGuardProtocol, usize)>,
}

#[derive(Debug)]
pub(crate) struct Output {
	pub ports: Vec<(GameGuardProtocol, Vec<u16>)>,
}

#[operation]
pub(crate) async fn pegboard_actor_allocate_ingress_ports(
	ctx: &OperationCtx,
	input: &Input,
) -> GlobalResult<Output> {
	let gg_config = &ctx.config().server()?.rivet.guard;

	let ports = ctx
		.fdb()
		.await?
		.run(|tx, _mc| async move {
			let mut results = Vec::new();

			// TODO: Parallelize
			for (protocol, count) in &input.ports {
				// Determine port range per protocol
				let port_range = match protocol {
					GameGuardProtocol::Http | GameGuardProtocol::Https => {
						return Err(fdb::FdbBindingError::CustomError(
							"Dynamic allocation not implemented for http/https ports".into(),
						));
					}
					GameGuardProtocol::Tcp | GameGuardProtocol::TcpTls => {
						gg_config.min_ingress_port_tcp()..=gg_config.max_ingress_port_tcp()
					}
					GameGuardProtocol::Udp => {
						gg_config.min_ingress_port_udp()..=gg_config.max_ingress_port_udp()
					}
				};

				let mut last_port = None;
				let mut ports = Vec::new();

				// Choose a random starting port for better spread and less cache hits
				let mut start = {
					// It is important that we don't start at the end of the range so that the logic with
					// `last_port` works correctly
					let exclusive_port_range = *port_range.start()..*port_range.end();
					rand::thread_rng().gen_range(exclusive_port_range)
				};

				// Build start and end keys for ingress ports subspace
				let start_key = keys::subspace()
					.subspace(&keys::port::IngressKey::subspace(*protocol, start))
					.range()
					.0;
				let end_key = keys::subspace()
					.subspace(&keys::port::IngressKey::subspace(
						*protocol,
						*port_range.end(),
					))
					.range()
					.1;
				let mut stream = tx.get_ranges_keyvalues(
					fdb::RangeOption {
						mode: StreamingMode::Iterator,
						..(start_key, end_key.clone()).into()
					},
					// NOTE: This is not SERIALIZABLE because we don't want to conflict with all of the keys,
					// just the one we choose
					SNAPSHOT,
				);

				// Continue iterating over the same stream until all of the required ports are found
				for _ in 0..*count {
					// Iterate through the subspace range until a port is found
					let port = loop {
						let Some(entry) = stream.try_next().await? else {
							match last_port {
								Some(port) if port == *port_range.end() => {
									// End of range reached, start a new range read from the beginning (wrap around)
									if start != *port_range.start() {
										last_port = None;

										let old_start = start;
										start = *port_range.start();

										let start_key = keys::subspace()
											.subspace(&keys::port::IngressKey::subspace(
												*protocol, start,
											))
											.range()
											.0;
										stream = tx.get_ranges_keyvalues(
											fdb::RangeOption {
												mode: StreamingMode::Iterator,
												limit: Some(old_start as usize),
												..(start_key, end_key.clone()).into()
											},
											// NOTE: This is not SERIALIZABLE because we don't want to conflict
											// with all of the keys, just the one we choose
											SNAPSHOT,
										);

										continue;
									} else {
										break None;
									}
								}
								// Return port after last port
								Some(last_port) => {
									break Some(last_port + 1);
								}
								// No ports were returned (range is empty)
								None => {
									break Some(start);
								}
							}
						};

						let key = keys::subspace()
							.unpack::<keys::port::IngressKey>(entry.key())
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;
						let current_port = key.port;

						if let Some(last_port) = last_port {
							// Gap found
							if current_port != last_port + 1 {
								break Some(last_port + 1);
							}
						}

						last_port = Some(current_port);
					};

					let Some(port) = port else {
						return Err(fdb::FdbBindingError::CustomError(
							format!("not enough {protocol} ports available").into(),
						));
					};

					let ingress_port_key =
						keys::port::IngressKey::new(*protocol, port, input.actor_id);
					let ingress_port_key_buf = keys::subspace().pack(&ingress_port_key);

					// Add read conflict only for this key
					tx.add_conflict_range(
						&ingress_port_key_buf,
						&end_of_key_range(&ingress_port_key_buf),
						ConflictRangeType::Read,
					)?;

					// Set key
					tx.set(
						&ingress_port_key_buf,
						&ingress_port_key
							.serialize(())
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
					);

					ports.push(port);
				}

				results.push((*protocol, ports));
			}

			Ok(results)
		})
		.custom_instrument(tracing::info_span!("allocate_ingress_ports_tx"))
		.await?;

	Ok(Output { ports })
}
