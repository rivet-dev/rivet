use rand::{thread_rng, Rng};

pub const IDENT_CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz1234567890-";
pub const IDENT_CHARSET_ALPHANUM: &[u8] = b"abcdefghijklmnopqrstuvwxyz1234567890";

/// Generates a random Rivet-safe identifier. See `check::ident`.
pub fn ident() -> String {
	let mut rng = thread_rng();
	let len = rng.gen_range((crate::check::MAX_IDENT_LEN - 4)..=crate::check::MAX_IDENT_LEN);

	// TODO: Use `rand::distributions::Uniform` and `.sample` instead of calling `gen_range` multiple times
	// Generate body including dash
	let body = std::iter::repeat_with(|| {
		let idx = rng.gen_range(0..IDENT_CHARSET.len());
		IDENT_CHARSET[idx] as char
	})
	.take(len - 2)
	.collect::<String>();
	let body = body.replace("--", "x-"); // Remove double-dashes

	// Choose alphanum characters for the start and finish since name IDs can't start or finish
	// with a dash
	let start = {
		let idx = rng.gen_range(0..IDENT_CHARSET_ALPHANUM.len());
		IDENT_CHARSET_ALPHANUM[idx] as char
	};
	let end = {
		let idx = rng.gen_range(0..IDENT_CHARSET_ALPHANUM.len());
		IDENT_CHARSET_ALPHANUM[idx] as char
	};

	format!("{}{}{}", start, body, end)
}

/// Generates a random Rivet-safe display name. See `check::display_name`.
pub fn display_name() -> String {
	let mut rng = thread_rng();
	let len = rng
		.gen_range((crate::check::MAX_DISPLAY_NAME_LEN - 4)..=crate::check::MAX_DISPLAY_NAME_LEN);
	rng.sample_iter(rand::distributions::Alphanumeric)
		.map(char::from)
		.take(len)
		.collect::<String>()
}

pub fn email() -> String {
	format!("test-{}@rivet.gg", ident())
}

pub fn bcrypt() -> (String, String) {
	let password = ident();

	(
		password.clone(),
		bcrypt::hash(password, bcrypt::DEFAULT_COST).unwrap(),
	)
}

pub fn ip_addr_v4() -> std::net::Ipv4Addr {
	let mut rng = thread_rng();
	std::net::Ipv4Addr::new(
		rng.gen::<u8>(),
		rng.gen::<u8>(),
		rng.gen::<u8>(),
		rng.gen::<u8>(),
	)
}

pub fn ip_addr_v6() -> std::net::Ipv6Addr {
	let mut rng = thread_rng();
	std::net::Ipv6Addr::new(
		rng.gen::<u16>(),
		rng.gen::<u16>(),
		rng.gen::<u16>(),
		rng.gen::<u16>(),
		rng.gen::<u16>(),
		rng.gen::<u16>(),
		rng.gen::<u16>(),
		rng.gen::<u16>(),
	)
}
