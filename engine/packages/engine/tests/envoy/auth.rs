use super::super::common;

use futures_util::StreamExt;
use tokio_tungstenite::{
	connect_async,
	tungstenite::{
		Message,
		client::IntoClientRequest,
		error::Error as WsError,
	},
};

fn envoy_connect_url(port: u16, namespace: &str, envoy_key: &str) -> String {
	format!(
		"ws://127.0.0.1:{}/envoys/connect?protocol_version={}&namespace={}&envoy_key={}&version=1&pool_name=test-envoy",
		port,
		common::test_envoy::PROTOCOL_VERSION,
		namespace,
		envoy_key
	)
}

#[test]
fn envoy_connect_rejects_bad_token() {
	common::run(
		common::TestOpts::new(1)
			.with_auth_admin_token("good-token")
			.with_timeout(20),
		|ctx| async move {
			let mut request =
				envoy_connect_url(ctx.leader_dc().guard_port(), "auth-namespace", "bad-token-envoy")
					.into_client_request()
					.expect("failed to create envoy connect request");
			request
				.headers_mut()
				.insert("Sec-WebSocket-Protocol", "rivet, rivet_token.bad-token".parse().unwrap());

			assert_envoy_rejection(request, "forbidden").await;
		},
	);
}

#[test]
fn envoy_connect_rejects_wrong_namespace() {
	common::run(common::TestOpts::new(1).with_timeout(20), |ctx| async move {
		let mut request = envoy_connect_url(
			ctx.leader_dc().guard_port(),
			"missing-namespace",
			"wrong-namespace-envoy",
		)
		.into_client_request()
		.expect("failed to create envoy connect request");
		request
			.headers_mut()
			.insert("Sec-WebSocket-Protocol", "rivet".parse().unwrap());

		assert_envoy_rejection(request, "namespace").await;
	});
}

#[test]
fn envoy_connect_rejects_invalid_envoy_key() {
	common::run(common::TestOpts::new(1).with_timeout(20), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;
		let mut request = envoy_connect_url(ctx.leader_dc().guard_port(), &namespace, "!!")
			.into_client_request()
			.expect("failed to create envoy connect request");
		request
			.headers_mut()
			.insert("Sec-WebSocket-Protocol", "rivet".parse().unwrap());

		assert_envoy_rejection(request, "invalid_url").await;
	});
}

async fn assert_envoy_rejection(
	request: tokio_tungstenite::tungstenite::http::Request<()>,
	expected_reason_fragment: &str,
) {
	match connect_async(request).await {
		Ok((mut ws, _)) => {
			let msg = tokio::time::timeout(std::time::Duration::from_secs(5), ws.next())
				.await
				.expect("timed out waiting for envoy rejection")
				.expect("envoy websocket should close after rejection")
				.expect("envoy websocket close should not error");
			match msg {
				Message::Close(Some(frame)) => {
					assert!(
						frame.reason.contains(expected_reason_fragment),
						"close reason should mention {expected_reason_fragment:?}, got {:?}",
						frame.reason
					);
				}
				other => panic!("expected envoy rejection close frame, got {other:?}"),
			}
		}
		Err(WsError::Http(response)) => {
			assert!(
				!response.status().is_success(),
				"envoy rejection should not be successful"
			);
		}
		Err(err) => {
			let message = err.to_string();
			assert!(
				message.contains(expected_reason_fragment),
				"envoy rejection should mention {expected_reason_fragment:?}, got {message}"
			);
		}
	}
}
