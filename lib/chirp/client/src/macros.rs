#[macro_export]
macro_rules! rpc {
	([$container:expr] $mod:ident { $($name:ident: $value:expr),* $(,)? }) => {
		$container
			.chirp()
			.rpc::<$mod::Endpoint>(
				None,
				#[allow(clippy::needless_update)]
				$mod::Request {
					$($name: $value,)*
					..Default::default()
				},
				false,
			)
	};
	([$container:expr] @dont_log_body $mod:ident { $($name:ident: $value:expr,)* }) => {
		$container
			.chirp()
			.rpc::<$mod::Endpoint>(
				None,
				#[allow(clippy::needless_update)]
				$mod::Request {
					$($name: $value,)*
					..Default::default()
				},
				true,
			)
	};
	([$container:expr] $mod:ident { $($name:ident: $value:expr,)* ..Default::default() }) => {
		op!([$container] $mod { $($name: $value,)* })
	};
}

// TODO: @notrace is not very clean, improve this
#[macro_export]
macro_rules! msg {
	(
		[$container:expr] $mod1:ident$(::$mod2:ident)* ($($param:expr),*)
		$body:tt
	) => {
		$container
			.chirp()
			.message::<$mod1$(::$mod2)*::Message>(
				$mod1$(::$mod2)*::build_params($($param),*),
				$mod1$(::$mod2)*::Message $body,
				Default::default(),
			)
	};

	(
		[$container:expr] @wait $mod1:ident$(::$mod2:ident)* ($($param:expr),*)
		$body:tt
	) => {
		$container
			.chirp()
			.message_wait::<$mod1$(::$mod2)*::Message>(
				$mod1$(::$mod2)*::build_params($($param),*),
				$mod1$(::$mod2)*::Message $body,
				Default::default(),
			)
	};

	(
		[$container:expr] @dont_log_body $mod1:ident$(::$mod2:ident)* ($($param:expr),*)
		$body:tt
	) => {
		$container
			.chirp()
			.message::<$mod1$(::$mod2)*::Message>(
				$mod1$(::$mod2)*::build_params($($param),*),
				$mod1$(::$mod2)*::Message $body,
				::chirp_client::MessageOptions {
					dont_log_body: true,
					..Default::default()
				},
			)
	};

	(
		[$container:expr] @recursive $mod1:ident$(::$mod2:ident)* ($($param:expr),*)
		$body:tt
	) => {
		$container
			.chirp()
			.message::<$mod1$(::$mod2)*::Message>(
				$mod1$(::$mod2)*::build_params($($param),*),
				$mod1$(::$mod2)*::Message $body,
				::chirp_client::MessageOptions {
					allow_recursive: true,
					..Default::default()
				},
			)
	};

	// message_with_subscribe w/ custom params
	(
		[$container:expr] $mod1:ident$(::$mod2:ident)* ($($param1:expr),*)
		-> $mod_complete1:ident$(::$mod_complete2:ident)* ($($param2:expr),*)
		$body:tt
	) => {
		$container
			.chirp()
			.message_with_subscribe::<
				$mod1$(::$mod2)*::Message,
				$mod_complete1$(::$mod_complete2)*::Message
			>(
				$mod1$(::$mod2)*::build_params($($param1),*),
				$mod1$(::$mod2)*::Message $body,
				Some($mod_complete1$(::$mod_complete2)*::build_params($($param2),*)),
				true,
			)
	};
	(
		[$container:expr] @notrace $mod1:ident$(::$mod2:ident)* ($($param1:expr),*)
		-> $mod_complete1:ident$(::$mod_complete2:ident)* ($($param2:expr),*)
		$body:tt
	) => {
		$container
			.chirp()
			.message_with_subscribe::<
				$mod1$(::$mod2)*::Message,
				$mod_complete1$(::$mod_complete2)*::Message
			>(
				$mod1$(::$mod2)*::build_params($($param1),*),
				$mod1$(::$mod2)*::Message $body,
				Some($mod_complete1$(::$mod_complete2)*::build_params($($param2),*)),
				false,
			)
	};

	// message_with_subscribe w/ implicit params
	(
		[$container:expr] $mod1:ident$(::$mod2:ident)* ($($param:expr),*)
		-> $mod_complete1:ident$(::$mod_complete2:ident)*
		{ $($name:ident: $value:expr),* $(,)? }
	) => {
		$container
			.chirp()
			.message_with_subscribe::<
				$mod1$(::$mod2)*::Message,
				$mod_complete1$(::$mod_complete2)*::Message
			>(
				$mod1$(::$mod2)*::build_params($($param),*),
				$mod1$(::$mod2)*::Message {
					$($name: $value,)*
				},
				None,
				true,
			)
	};
	// message_with_subscribe w/ implicit params, and default
	(
		[$container:expr] $mod1:ident$(::$mod2:ident)* ($($param:expr),*)
		-> $mod_complete1:ident$(::$mod_complete2:ident)*
		{ $($name:ident: $value:expr,)* ..Default::default() }
	) => {
		$container
			.chirp()
			.message_with_subscribe::<
				$mod1$(::$mod2)*::Message,
				$mod_complete1$(::$mod_complete2)*::Message
			>(
				$mod1$(::$mod2)*::build_params($($param),*),
				$mod1$(::$mod2)*::Message {
					$($name: $value,)*
					..Default::default()
				},
				None,
				true,
			)
	};
	(
		[$container:expr] @notrace $mod1:ident$(::$mod2:ident)* ($($param:expr),*)
		-> $mod_complete1:ident$(::$mod_complete2:ident)*
		{ $($name:ident: $value:expr),* $(,)? }
	) => {
		$container
			.chirp()
			.message_with_subscribe::<
				$mod1$(::$mod2)*::Message,
				$mod_complete1$(::$mod_complete2)*::Message
			>(
				$mod1$(::$mod2)*::build_params($($param),*),
				$mod1$(::$mod2)*::Message {
					$($name: $value,)*
				},
				None,
				false,
			)
	};

	// message_with_result
	(
		[$container:expr] $mod1:ident$(::$mod2:ident)* ($($param:expr),*)
		-> Result< $mod_ok1:ident$(::$mod_ok2:ident)*, $mod_err1:ident$(::$mod_err2:ident)* >
		$body:tt
	) => {
		$container
			.chirp()
			.message_with_result::<
				$mod1$(::$mod2)*::Message,
				$mod_ok1$(::$mod_ok2)*::Message,
				$mod_err1$(::$mod_err2)*::Message
			>(
				$mod1$(::$mod2)*::build_params($($param),*),
				$mod1$(::$mod2)*::Message $body,
				None,
				None,
				true,
			)
	};
	(
		[$container:expr] @notrace $mod1:ident$(::$mod2:ident)* ($($param:expr),*)
		-> Result< $mod_ok1:ident$(::$mod_ok2:ident)*, $mod_err1:ident$(::$mod_err2:ident)* >
		$body:tt
	) => {
		$container
			.chirp()
			.message_with_result::<
				$mod1$(::$mod2)*::Message,
				$mod_ok1$(::$mod_ok2)*::Message,
				$mod_err1$(::$mod_err2)*::Message
			>(
				$mod1$(::$mod2)*::build_params($($param),*),
				$mod1$(::$mod2)*::Message $body,
				None,
				None,
				false,
			)
	};
}

/// Small wrapper around a rust operation call to provide context and handle trace.
#[macro_export]
macro_rules! op {
	([$container:expr] $mod:ident { $($name:ident: $value:expr),* $(,)? }) => {
		$container.op_ctx().call::<::$mod::Operation>(
			#[allow(clippy::needless_update)]
			::$mod::__Request {
				$($name: $value,)*
				..Default::default()
			},
		)
	};
	([$container:expr] $mod:ident { $($name:ident: $value:expr,)* ..Default::default() }) => {
		op!([$container] $mod { $($name: $value,)* })
	};

	// TODO: Does nothing different
	([$container:expr] @dont_log_body $($t:tt)*) => {
		op!([$container] $($t)+)
	};
}

/// There is no `subscribe_one` because the `subscribe` is usually earlier in
/// the code from where the response is consumed. Using this macro usually means
/// there is a race condition somewhere.
#[macro_export]
macro_rules! subscribe {
	([$container:expr] $mod1:ident$(::$mod2:ident)* ($($param:expr),*)) => {
		$container
			.chirp()
			.subscribe::<$mod1$(::$mod2)*::Message>(
				$mod1$(::$mod2)*::build_params($($param),*)
			)
	};
}

#[macro_export]
macro_rules! tail_read {
	([$container:expr] $mod1:ident$(::$mod2:ident)* ($($param:expr),*)) => {
		{
			let params = $mod1$(::$mod2)*::build_params($($param),*);
			tracing::Instrument::instrument(
				async {
					$container
						.chirp()
						.tail_read::<$mod1$(::$mod2)*::Message>(params)
						.await
				},
				tracing::info_span!("tail_read"),
			)
		}
	};
}

#[macro_export]
macro_rules! tail_anchor {
	([$container:expr, $anchor:expr] $mod1:ident$(::$mod2:ident)* ($($param:expr),*)) => {
		{
			let params = $mod1$(::$mod2)*::build_params($($param),*);
			tracing::Instrument::instrument(
				async {
					$container
						.chirp()
						.tail_anchor::<$mod1$(::$mod2)*::Message>(params, &$anchor)
						.await
				},
				tracing::info_span!("tail_anchor"),
			)
		}
	};
}

#[macro_export]
macro_rules! tail_all {
	([$container:expr, $anchor:expr, $config:expr] $mod1:ident$(::$mod2:ident)* ($($param:expr),*)) => {
		{
			let params = $mod1$(::$mod2)*::build_params($($param),*);
			$container
				.chirp()
				.tail_all::<$mod1$(::$mod2)*::Message>(vec![params], &$anchor, $config)
		}
	};
}

// TODO: Add back
// #[cfg(test)]
// mod tests {
// 	struct EmptyHolder;

// 	impl EmptyHolder {
// 		fn chirp(&self) -> crate::Client {
// 			panic!()
// 		}
// 	}

// 	mod test_endpoint {
// 		pub struct Endpoint;

// 		impl crate::endpoint::Endpoint for Endpoint {
// 			type Request = Request;
// 			type Response = Response;
// 			const NAME: &'static str = "EMPTY";
// 			const TIMEOUT: std::time::Duration = Duration::from_secs(30);
// 		}

// 		#[derive(Clone, PartialEq, prost::Message)]
// 		pub struct Request {
// 			#[prost(int32, tag = "1")]
// 			pub a: i32,
// 			#[prost(string, tag = "2")]
// 			pub b: String,
// 		}

// 		#[derive(Clone, PartialEq, prost::Message)]
// 		pub struct Response {}
// 	}

// 	#[test]
// 	#[ignore]
// 	fn test_rpc_macro() {
// 		let holder = EmptyHolder;
// 		let _ = op!([holder] test_endpoint {
// 			a: 5,
// 			b: String::new(),
// 		});
// 	}

// 	mod test_message {
// 		#[derive(Clone, PartialEq, prost::Message)]
// 		pub struct Message {
// 			#[prost(int32, tag = "1")]
// 			pub a: i32,
// 			#[prost(string, tag = "2")]
// 			pub b: String,
// 		}
// 	}

// 	#[test]
// 	#[ignore]
// 	fn test_message_macro() {
// 		let holder = EmptyHolder;
// 		msg!([holder] test_message("abc", 123) {
// 			a: 5,
// 			b: String::new(),
// 		});
// 	}
