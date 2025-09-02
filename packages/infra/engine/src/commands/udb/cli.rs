use std::{
	result::Result::{Err, Ok},
	time::Duration,
};

use clap::{Parser, ValueEnum};
use futures_util::TryStreamExt;
use rivet_pools::UdbPool;
use rivet_term::console::style;
use universaldb::{self as udb, options::StreamingMode};

use crate::util::{
	format::indent_string,
	udb::{ListStyle, SimpleTuple, SimpleTupleValue},
};

// TODO: Tab completion
#[derive(Parser)]
#[command(name = "")]
pub enum SubCommand {
	/// Change current key.
	#[command(name = "cd")]
	ChangeKey {
		/// Key path to change to. Supports relative key paths.
		key: String,
	},

	/// Get value at current key.
	#[command(name = "get")]
	Get {
		/// Key path to get. Supports relative key paths.
		key: Option<String>,

		/// Optional type hint for value parsing.
		#[arg(short = 't', long = "type")]
		type_hint: Option<String>,
	},

	/// List all keys under the current key.
	#[command(name = "ls")]
	List {
		/// Key path to list. Supports relative key paths.
		key: Option<String>,

		/// Max depth of keys shown.
		#[arg(short = 'd', long, default_value_t = 1)]
		max_depth: usize,
		/// Whether or not to hide subspace keys which are past the max depth.
		#[arg(short = 'u', long, default_value_t = false)]
		hide_subspaces: bool,
		/// Print style
		#[arg(short = 's', long, default_value = "tree")]
		style: ListStyle,
	},

	/// Move single key or entire subspace from A to B.
	#[command(name = "move")]
	Move {
		/// Old key path. Supports relative key paths.
		old_key: String,

		/// New key path. Supports relative key paths.
		new_key: String,

		/// Move all keys under the given key instead of just the given key.
		#[arg(short = 'r', long)]
		recursive: bool,
	},

	/// Set value at current path.
	#[command(name = "set")]
	Set {
		/// Key path to set. Supports relative key paths. Used as value if value is not set.
		key_or_value: String,

		/// Value to set, with optional type prefix (e.g. "u64:123", overrides type hint).
		value: Option<String>,

		/// Optional type hint for the value.
		#[arg(short = 't', long)]
		type_hint: Option<String>,
	},

	/// Clear value or range at current path.
	#[command(name = "clear")]
	Clear {
		/// Key path to clear. Supports relative key paths.
		key: Option<String>,

		/// Clears the entire subspace range instead of just the key.
		#[arg(short = 'r', long = "range", default_value_t = false)]
		clear_range: bool,
		/// Disable confirmation prompt.
		#[arg(short = 'y', long, default_value_t = false)]
		yes: bool,
	},

	#[command(name = "exit")]
	Exit,
}

impl SubCommand {
	pub async fn execute(
		self,
		pool: &UdbPool,
		previous_tuple: &mut SimpleTuple,
		current_tuple: &mut SimpleTuple,
	) -> CommandResult {
		match self {
			SubCommand::ChangeKey { key } => {
				if key.trim() == "-" {
					let other = current_tuple.clone();
					*current_tuple = previous_tuple.clone();
					*previous_tuple = other;
				} else {
					*previous_tuple = current_tuple.clone();
					update_current_tuple(current_tuple, Some(key));
				}
			}
			SubCommand::Get { key, type_hint } => {
				let mut current_tuple = current_tuple.clone();
				if update_current_tuple(&mut current_tuple, key) {
					return CommandResult::Error;
				}

				let fut = pool.run(|tx, _mc| {
					let current_tuple = current_tuple.clone();
					async move {
						let key = udb::tuple::pack(&current_tuple);
						let entry = tx.get(&key, true).await?;
						Ok(entry)
					}
				});

				match tokio::time::timeout(Duration::from_secs(5), fut).await {
					Ok(Ok(entry)) => {
						if let Some(entry) = entry {
							if type_hint.as_deref() == Some("raw") {
								println!("{}", SimpleTupleValue::Unknown(entry.to_vec()));
							} else {
								match SimpleTupleValue::deserialize(type_hint.as_deref(), &entry) {
									Ok(parsed) => {
										let mut s = String::new();
										parsed.write(&mut s, false).unwrap();
										println!("{s}");
									}
									Err(err) => println!("error: {err:#}"),
								}
							}
						} else {
							println!("key does not exist");
						};
					}
					Ok(Err(err)) => println!("txn error: {err:#}"),
					Err(_) => println!("txn timed out"),
				}
			}
			SubCommand::List {
				key,
				max_depth,
				hide_subspaces,
				style: list_style,
			} => {
				let mut current_tuple = current_tuple.clone();
				if update_current_tuple(&mut current_tuple, key) {
					return CommandResult::Error;
				}

				let subspace = udb::tuple::Subspace::all().subspace(&current_tuple);

				let fut = pool.run(|tx, _mc| {
					let subspace = subspace.clone();
					async move {
						let entries = tx
							.get_ranges_keyvalues(
								udb::RangeOption {
									mode: StreamingMode::WantAll,
									..(&subspace).into()
								},
								true,
							)
							.try_collect::<Vec<_>>()
							.await?;

						Ok(entries)
					}
				});

				match tokio::time::timeout(Duration::from_secs(5), fut).await {
					Ok(Ok(entries)) => {
						let mut entry_count = 0;
						let mut subspace_count = 0;
						let mut current_hidden_subspace: Option<SimpleTuple> = None;
						let mut hidden_count = 0;
						let mut last_key = SimpleTuple::new();

						for entry in &entries {
							match subspace.unpack::<SimpleTuple>(entry.key()) {
								Ok(key) => {
									if key.segments.len() <= max_depth {
										if entry.value().is_empty() {
											key.print(&list_style, &last_key);
											println!();
										} else {
											match SimpleTupleValue::deserialize(None, entry.value())
											{
												Ok(value) => {
													let mut s = String::new();
													value.write(&mut s, false).unwrap();

													key.print(&list_style, &last_key);

													let indent = match list_style {
														ListStyle::List => "  ".to_string(),
														ListStyle::Tree => format!(
															"  {}",
															"| ".repeat(key.segments.len()),
														),
													};
													println!(
														" = {}",
														indent_string(
															&s,
															style(indent).dim().to_string(),
															true
														)
													);
												}
												Err(err) => {
													key.print(&list_style, &last_key);
													println!(" error: {err:#}");
												}
											}
										}

										last_key = key;

										if let Some(curr) = &current_hidden_subspace {
											curr.print(&list_style, &last_key);
											println!(
												"/ {}",
												style(format!(
													"{hidden_count} {}",
													if hidden_count == 1 {
														"entry"
													} else {
														"entries"
													}
												))
												.dim()
											);

											last_key = curr.clone();
											current_hidden_subspace = None;
											hidden_count = 0;
										}

										entry_count += 1;
									} else if !hide_subspaces {
										let sliced = key.slice(max_depth);

										if let Some(curr) = &current_hidden_subspace {
											if &sliced == curr {
												hidden_count += 1;
											} else {
												curr.print(&list_style, &last_key);
												println!(
													"/ {}",
													style(format!(
														"{hidden_count} {}",
														if hidden_count == 1 {
															"entry"
														} else {
															"entries"
														}
													))
													.dim()
												);

												last_key = curr.clone();
												current_hidden_subspace = Some(sliced);
												hidden_count = 1;
												subspace_count += 1;
											}
										} else {
											current_hidden_subspace = Some(sliced);
											hidden_count = 1;
											subspace_count += 1;
										}
									}
								}
								Err(err) => println!("error parsing key: {err:#}"),
							}
						}

						if !hide_subspaces {
							if let Some(curr) = current_hidden_subspace {
								curr.print(&list_style, &last_key);
								println!(
									"/ {}",
									style(format!(
										"{hidden_count} {}",
										if hidden_count == 1 {
											"entry"
										} else {
											"entries"
										}
									))
									.dim()
								);
							}
						}

						if !entries.is_empty() {
							println!();
						}

						print!(
							"{} {}",
							entry_count,
							if entry_count == 1 { "entry" } else { "entries" }
						);

						if subspace_count != 0 {
							print!(
								", {} {} ({} total entries)",
								subspace_count,
								if subspace_count == 1 {
									"subspace"
								} else {
									"subspaces"
								},
								entries.len()
							);
						}

						println!();
					}
					Ok(Err(err)) => println!("txn error: {err:#}"),
					Err(_) => println!("txn timed out"),
				}
			}
			SubCommand::Move {
				old_key,
				new_key,
				recursive,
			} => {
				let mut old_tuple = current_tuple.clone();
				if update_current_tuple(&mut old_tuple, Some(old_key)) {
					return CommandResult::Error;
				}

				let mut new_tuple = current_tuple.clone();
				if update_current_tuple(&mut new_tuple, Some(new_key)) {
					return CommandResult::Error;
				}

				let fut = pool.run(|tx, _mc| {
					let old_tuple = old_tuple.clone();
					let new_tuple = new_tuple.clone();
					async move {
						if recursive {
							let old_subspace = udb::tuple::Subspace::all().subspace(&old_tuple);
							let new_subspace = udb::tuple::Subspace::all().subspace(&new_tuple);

							// Get all key-value pairs from the old subspace
							let mut stream = tx.get_ranges_keyvalues(
								udb::RangeOption {
									mode: StreamingMode::WantAll,
									..(&old_subspace).into()
								},
								true,
							);

							let mut keys_moved = 0;
							while let Some(entry) = stream.try_next().await? {
								// Unpack key from old subspace
								if let Ok(relative_tuple) =
									old_subspace.unpack::<SimpleTuple>(entry.key())
								{
									// Create new key in the new subspace
									let new_key = new_subspace.pack(&relative_tuple);

									// Set value at new key and clear old key
									tx.set(&new_key, entry.value());
									tx.clear(entry.key());

									keys_moved += 1;
								} else {
									eprintln!("failed unpacking key");
								}
							}

							Ok(keys_moved)
						} else {
							let old_key = udb::tuple::pack(&old_tuple);
							let new_key = udb::tuple::pack(&new_tuple);

							let Some(value) = tx.get(&old_key, true).await? else {
								return Ok(0);
							};

							tx.set(&new_key, &value);
							tx.clear(&old_key);

							Ok(1)
						}
					}
				});

				match tokio::time::timeout(Duration::from_secs(5), fut).await {
					Ok(Ok(keys_moved)) => {
						if keys_moved == 0 && !recursive {
							println!("key does not exist");
						} else {
							println!(
								"{} key{} moved",
								keys_moved,
								if keys_moved == 1 { "" } else { "s" }
							);
						}
					}
					Ok(Err(err)) => println!("txn error: {err:#}"),
					Err(_) => println!("txn timed out"),
				}
			}
			SubCommand::Set {
				key_or_value,
				value,
				type_hint,
			} => {
				let (key, value) = if let Some(value) = value {
					(Some(key_or_value), value)
				} else {
					(None, key_or_value)
				};

				let mut current_tuple = current_tuple.clone();
				if update_current_tuple(&mut current_tuple, key) {
					return CommandResult::Error;
				}

				let parsed_value = match SimpleTupleValue::parse_str_with_type_hint(
					type_hint.as_deref(),
					&value,
				) {
					Ok(parsed) => parsed,
					Err(err) => {
						println!("error: {err:#}");
						return CommandResult::Error;
					}
				};

				let fut = pool.run(|tx, _mc| {
					let current_tuple = current_tuple.clone();
					let value = parsed_value.clone();
					async move {
						let key = udb::tuple::pack(&current_tuple);
						let value = value
							.serialize()
							.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

						tx.set(&key, &value);
						Ok(())
					}
				});

				match tokio::time::timeout(Duration::from_secs(5), fut).await {
					Ok(Ok(_)) => {}
					Ok(Err(err)) => println!("txn error: {err:#}"),
					Err(_) => println!("txn timed out"),
				}
			}
			SubCommand::Clear {
				key,
				clear_range,
				yes,
			} => {
				let mut current_tuple = current_tuple.clone();
				if update_current_tuple(&mut current_tuple, key) {
					return CommandResult::Error;
				}

				if !yes {
					let term = rivet_term::terminal();
					let response = rivet_term::prompt::PromptBuilder::default()
						.message("Are you sure?")
						.build()
						.expect("failed to build prompt")
						.bool(&term)
						.await
						.expect("failed to show prompt");
					if !response {
						return CommandResult::Error;
					}
				}

				let fut = pool.run(|tx, _mc| {
					let current_tuple = current_tuple.clone();
					async move {
						if clear_range {
							let subspace = udb::tuple::Subspace::all().subspace(&current_tuple);
							tx.clear_subspace_range(&subspace);
						} else {
							let key = udb::tuple::pack(&current_tuple);
							tx.clear(&key);
						}

						Ok(())
					}
				});

				match tokio::time::timeout(Duration::from_secs(5), fut).await {
					Ok(Ok(_)) => {}
					Ok(Err(err)) => println!("txn error: {err:#}"),
					Err(_) => println!("txn timed out"),
				}
			}
			SubCommand::Exit => return CommandResult::Exit,
		}

		CommandResult::Ok
	}
}

#[derive(Debug, ValueEnum, Clone, Copy, PartialEq)]
pub enum ClearType {
	Range,
}

pub enum CommandResult {
	Ok,
	Error,
	Exit,
}

fn update_current_tuple(current_tuple: &mut SimpleTuple, key: Option<String>) -> bool {
	let Some(key) = key.as_deref() else {
		return false;
	};

	match SimpleTuple::parse(key) {
		Ok((parsed, relative, back_count)) => {
			if relative {
				for _ in 0..back_count {
					current_tuple.segments.pop();
				}
				current_tuple.segments.extend(parsed.segments);
			} else {
				*current_tuple = parsed;
			}

			false
		}
		Err(err) => {
			println!("error: {err:#}");

			true
		}
	}
}
