use include_dir::{include_dir, Dir};

const HUB_DIR: Dir = include_dir!("$HUB_PATH");

pub fn get_file_content(path: &str) -> Option<&'static [u8]> {
	HUB_DIR.get_file(path).map(|file| file.contents())
}
