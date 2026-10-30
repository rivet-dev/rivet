use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use rivet_util::net::bind_tcp_listener;
use tokio::net::TcpStream;

#[tokio::test]
async fn ipv6_unspecified_listener_accepts_ipv4_loopback_connections() {
	assert_accepts_ipv4_loopback_connection(Ipv6Addr::UNSPECIFIED.into()).await;
}

#[tokio::test]
async fn ipv4_listener_accepts_ipv4_loopback_connections() {
	assert_accepts_ipv4_loopback_connection(Ipv4Addr::LOCALHOST.into()).await;
}

async fn assert_accepts_ipv4_loopback_connection(listen_ip: IpAddr) {
	let listener = bind_tcp_listener(SocketAddr::new(listen_ip, 0)).expect("bind listener");
	let port = listener.local_addr().expect("listener address").port();

	TcpStream::connect((Ipv4Addr::LOCALHOST, port))
		.await
		.expect("connect over ipv4 loopback");
	listener.accept().await.expect("accept ipv4 connection");
}
