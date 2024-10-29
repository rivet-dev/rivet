use include_dir::{include_dir, Dir};

const HUB_DIR: Dir = include_dir!("$HUB_PATH");

pub fn get_file_content(path: &str) -> Option<&'static str> {
	HUB_DIR.get_file(path).and_then(|file| file.contents_utf8())
}
