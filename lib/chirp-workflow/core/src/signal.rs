pub trait Signal {
	const NAME: &'static str;
}

/// Creates an enum that implements `Listen` and selects one of X signals.
///
/// Example:
/// ```rust
/// #[macros::signal("my-signal")]
/// struct MySignal {
/// 	x: i64,
/// }

/// #[macros::signal("my-signal2")]
/// struct MySignal2 {
/// 	y: i64,
/// }
///
/// join_signal!(MyJoinSignal, [MySignal, MySignal2]);
///
/// // Automatically becomes:
/// enum MyJoinSignal {
/// 	MySignal(MySignal),
/// 	MySignal2(MySignal2),
/// }
///
/// // Listening:
/// match ctx.listen::<MyJoinSignal>() {
/// 	MySignal(sig) => println!("received MySignal {sig:?}"),
/// 	MySignal2(sig) => println!("received MySignal2 {sig:?}"),
/// }
/// ````
#[macro_export]
macro_rules! join_signal {
	($vis:vis $join:ident, [$($signals:ident),* $(,)?]) => {
		$vis enum $join {
			$($signals($signals)),*
		}

		#[async_trait::async_trait]
		impl Listen for $join {
			async fn listen(ctx: &chirp_workflow::prelude::ListenCtx) -> chirp_workflow::prelude::WorkflowResult<Self> {
				let row = ctx.listen_any(&[
					$(<$signals as chirp_workflow::prelude::Signal>::NAME),*
				]).await?;

				Self::parse(&row.signal_name, row.body)
			}

			fn parse(name: &str, body: serde_json::Value) -> chirp_workflow::prelude::WorkflowResult<Self> {
				$(
					if name == <$signals as chirp_workflow::prelude::Signal>::NAME {
						Ok(
							Self::$signals(
								serde_json::from_value(body)
									.map_err(WorkflowError::DeserializeSignalBody)?
							)
						)
					}
				)else*

				else {
					unreachable!(
						"received signal that wasn't queried for: {}, expected {:?}",
						name, &[$(<$signals as chirp_workflow::prelude::Signal>::NAME),*]
					);
				}
			}
		}
	};
}
pub use join_signal;
