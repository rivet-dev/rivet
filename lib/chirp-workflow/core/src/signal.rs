pub trait Signal {
	const NAME: &'static str;
}

/// Creates an enum that implements `Listen` and selects one of X signals.
///
/// Example:
/// ```rust
/// #[signal("my_signal")]
/// struct MySignal {
/// 	x: i64,
/// }

/// #[signal("my_signal2")]
/// struct MySignal2 {
/// 	y: i64,
/// }
///
/// join_signal!(MyJoinSignal {
/// 	MySignal,
/// 	MySignal2
/// });
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
///
///
/// // Also allows aliases:
/// join_signal!(MyJoinSignal {
/// 	MySignal,
/// 	MySignal2(some_pkg::Signal),
/// }
/// ```
#[macro_export]
macro_rules! join_signal {
	($vis:vis $join:ident { $($tt:tt)* }) => {
		join_signal!(@ $vis $join [] [] $($tt)*);
	};
	(@
	    $vis:vis $join:ident
	    [$({ $names:tt } { $types:tt })*]
	    [$({ $just_types:tt })*]
	) => {
    	$vis enum $join {
			$( $names ($types) ),*
		}

    	#[async_trait::async_trait]
		impl Listen for $join {
			async fn listen(ctx: &chirp_workflow::prelude::ListenCtx) -> chirp_workflow::prelude::WorkflowResult<Self> {
				let row = ctx.listen_any(&[
				    $(<$just_types as chirp_workflow::prelude::Signal>::NAME),*
				]).await?;

				Self::parse(&row.signal_name, row.body)
			}

			fn parse(name: &str, body: serde_json::Value) -> chirp_workflow::prelude::WorkflowResult<Self> {
				$(
				    if name == <$types as chirp_workflow::prelude::Signal>::NAME {
						Ok(
							Self::$names(
								serde_json::from_value(body)
									.map_err(WorkflowError::DeserializeSignalBody)?
							)
						)
					}
				)else*
				else {
					unreachable!(
						"received signal that wasn't queried for: {}, expected {:?}",
						name, &[$(<$just_types as chirp_workflow::prelude::Signal>::NAME),*]
					);
				}
			}
		}
    };
	(@
	    $vis:vis $join:ident
	    [$({ $names:tt } { $types:tt })*]
	    [$({ $just_types:tt })*]
	    $name:ident,
	    $($tail:tt)*
	) => {
	   join_signal!(@
	       $vis $join
	       [$( { $names } { $types } )* { $name } { $name }]
	       [$( { $just_types } )* { $name }]
	       $($tail)*
	   );
	};
	(@
	    $vis:vis $join:ident
	    [$({ $names:tt } { $types:tt })*]
	    [$({ $just_types:tt })*]
	    $name:ident($ty:ty),
	    $($tail:tt)*
	) => {
	   join_signal!(@
	       $vis $join
	       [$( { $names } { $types } )* { $name } { $ty }]
	       [$( { $just_types } )* { $ty }]
	       $($tail)*
	   );
	};
}
pub use join_signal;
