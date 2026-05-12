// Rtt subcommand: spawn workers that open a raw WS, send "1" + "2", and
// measure connect/first/second/total latency.

use std::sync::Arc;
use std::time::Instant;

use futures_util::{SinkExt, StreamExt};
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

use crate::args::{EnvConfig, RttArgs};
use crate::endpoint::Endpoint;
use crate::log::{DIM, RED, RESET, color_ms, format_actor, iso_now, pad};
use crate::ws::open_raw_ws;

pub struct Sample {
	pub worker: u32,
	pub key: String,
	pub connect_ms: f64,
	pub first_ms: f64,
	pub second_ms: f64,
	pub total_ms: f64,
	pub actor_id: Option<String>,
	pub error: Option<String>,
}

pub async fn run_rtt_mode(args: RttArgs, env: Arc<EnvConfig>, endpoint: Arc<Endpoint>) {
	let mut worker_id: u32 = 0;
	let mut inflight: Vec<tokio::task::JoinHandle<()>> = Vec::new();
	loop {
		if env.batches != 0 && (worker_id as u64) >= env.batches {
			break;
		}
		worker_id += 1;
		let id = worker_id;
		let endpoint_clone = endpoint.clone();
		let skip_ready_wait = args.skip_ready_wait;
		if env.serial {
			let sample = run_rtt_worker(id, endpoint_clone, skip_ready_wait).await;
			print_rtt_sample(&sample);
		} else {
			let handle = tokio::spawn(async move {
				let sample = run_rtt_worker(id, endpoint_clone, skip_ready_wait).await;
				print_rtt_sample(&sample);
			});
			inflight.push(handle);
		}
		if env.batches == 0 || (worker_id as u64) < env.batches {
			sleep(std::time::Duration::from_millis(args.interval)).await;
		}
	}
	for h in inflight {
		let _ = h.await;
	}
}

async fn run_rtt_worker(worker: u32, endpoint: Arc<Endpoint>, skip_ready_wait: bool) -> Sample {
	let key = make_key(worker, "cl");
	let actor_id: Option<String> = None;
	let url = endpoint.build_raw_ws_url("counter", &key, skip_ready_wait);
	let t0 = Instant::now();

	match open_and_run_rtt(&url, t0).await {
		Ok((connect_ms, first_ms, second_ms, total_ms)) => Sample {
			worker,
			key,
			connect_ms,
			first_ms,
			second_ms,
			total_ms,
			actor_id,
			error: None,
		},
		Err(err) => Sample {
			worker,
			key,
			connect_ms: 0.0,
			first_ms: 0.0,
			second_ms: 0.0,
			total_ms: 0.0,
			actor_id,
			error: Some(err.to_string()),
		},
	}
}

async fn open_and_run_rtt(url: &str, t0: Instant) -> anyhow::Result<(f64, f64, f64, f64)> {
	let ws = open_raw_ws(url).await?;
	let t_connect = Instant::now();
	let (mut sink, mut stream) = ws.split();

	sink.send(Message::Text("1".into())).await?;
	wait_for_echo(&mut stream).await?;
	let t_first = Instant::now();

	sink.send(Message::Text("2".into())).await?;
	wait_for_echo(&mut stream).await?;
	let t_second = Instant::now();

	let _ = sink
		.send(Message::Close(Some(CloseFrame {
			code: CloseCode::Normal,
			reason: "rtt done".into(),
		})))
		.await;

	let connect_ms = elapsed_ms(t0, t_connect);
	let first_ms = elapsed_ms(t_connect, t_first);
	let second_ms = elapsed_ms(t_first, t_second);
	let total_ms = elapsed_ms(t0, t_second);
	Ok((connect_ms, first_ms, second_ms, total_ms))
}

async fn wait_for_echo<S>(stream: &mut S) -> anyhow::Result<()>
where
	S: futures_util::stream::Stream<
			Item = Result<Message, tokio_tungstenite::tungstenite::Error>,
		> + Unpin,
{
	loop {
		match stream.next().await {
			Some(Ok(Message::Text(_) | Message::Binary(_))) => return Ok(()),
			Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_))) => continue,
			Some(Ok(Message::Close(frame))) => {
				let (code, reason) = match frame {
					Some(f) => (u16::from(f.code), f.reason.to_string()),
					None => (0, String::new()),
				};
				return Err(anyhow::anyhow!(
					"ws closed before echo code={} reason={}",
					code,
					reason
				));
			}
			Some(Err(e)) => return Err(anyhow::anyhow!("ws error before echo: {}", e)),
			None => return Err(anyhow::anyhow!("ws stream ended before echo")),
		}
	}
}

fn elapsed_ms(start: Instant, end: Instant) -> f64 {
	end.duration_since(start).as_secs_f64() * 1000.0
}

pub fn make_key(worker: u32, prefix: &str) -> String {
	let now_ms = chrono::Utc::now().timestamp_millis();
	format!("{}-{}-{}", prefix, worker, base36(now_ms as u64))
}

fn base36(mut n: u64) -> String {
	if n == 0 {
		return "0".to_string();
	}
	let mut s = String::new();
	const ALPHA: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
	while n > 0 {
		s.push(ALPHA[(n % 36) as usize] as char);
		n /= 36;
	}
	s.chars().rev().collect()
}

pub fn print_rtt_sample(s: &Sample) {
	let prefix = format!("{}{}{}", DIM, iso_now(), RESET);
	if let Some(err) = &s.error {
		crate::out!(
			"{} {} {}ERROR {}{} ({})",
			prefix,
			pad(&s.key, 32),
			RED,
			err,
			RESET,
			color_ms(s.total_ms),
		);
		return;
	}
	crate::out!(
		"{} {}{} connect={} first={} second={} total={}",
		prefix,
		pad(&s.key, 32),
		format_actor(s.actor_id.as_deref()),
		color_ms(s.connect_ms),
		color_ms(s.first_ms),
		color_ms(s.second_ms),
		color_ms(s.total_ms),
	);
}
