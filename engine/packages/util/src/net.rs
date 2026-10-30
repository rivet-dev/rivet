use std::{io, net::SocketAddr};

use socket2::{Domain, Protocol, Socket, Type};

const LISTEN_BACKLOG: i32 = 1024;

pub fn bind_tcp_listener(addr: SocketAddr) -> io::Result<tokio::net::TcpListener> {
	tokio::net::TcpListener::from_std(bind_std_tcp_listener(addr)?)
}

pub fn bind_std_tcp_listener(addr: SocketAddr) -> io::Result<std::net::TcpListener> {
	let socket = Socket::new(Domain::for_address(addr), Type::STREAM, Some(Protocol::TCP))?;

	if addr.is_ipv6() {
		socket.set_only_v6(false)?;
	}

	allow_fast_rebind(&socket)?;
	socket.bind(&addr.into())?;
	socket.listen(LISTEN_BACKLOG)?;
	socket.set_nonblocking(true)?;

	Ok(socket.into())
}

#[cfg(unix)]
fn allow_fast_rebind(socket: &Socket) -> io::Result<()> {
	socket.set_reuse_address(true)
}

#[cfg(not(unix))]
fn allow_fast_rebind(_socket: &Socket) -> io::Result<()> {
	Ok(())
}
