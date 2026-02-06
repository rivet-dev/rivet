pub const DEFAULT_CHUNK_SIZE: usize = 4096;
pub const SQLITE_PREFIX: u8 = 9;
pub const META_PREFIX: u8 = 0;
pub const CHUNK_PREFIX: u8 = 1;

pub fn meta_key(file_name: &str) -> Vec<u8> {
	let file_name_bytes = file_name.as_bytes();
	let mut key = Vec::with_capacity(2 + file_name_bytes.len());
	key.push(SQLITE_PREFIX);
	key.push(META_PREFIX);
	key.extend_from_slice(file_name_bytes);
	key
}

pub fn chunk_key(file_name: &str, chunk_index: u32) -> Vec<u8> {
	let file_name_bytes = file_name.as_bytes();
	let mut key = Vec::with_capacity(2 + file_name_bytes.len() + 1 + 4);
	key.push(SQLITE_PREFIX);
	key.push(CHUNK_PREFIX);
	key.extend_from_slice(file_name_bytes);
	key.push(0);
	key.extend_from_slice(&chunk_index.to_be_bytes());
	key
}
