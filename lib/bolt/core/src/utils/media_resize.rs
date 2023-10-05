const UUID_V4_REGEXP: &str = "[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}";

pub struct ResizePreset {
	ns: String,
	name: String,
	bucket: String,
	file_path: String,
	format: ResizeFormat,
	size: (usize, usize),
	scale: ResizeScale,
	game_cors: bool,
	/// If true, this path will be used as a fallback when there's not resizing
	/// options are specified.
	fallback: bool,
}

impl ResizePreset {
	fn key(&self) -> String {
		let name = &self.name;
		let path = self.file_path.replace("/", "-").replace(".", "-");
		if self.fallback {
			format!("{name}-{path}-fallback")
		} else {
			let format = self.format.short();
			let w = self.size.0;
			let h = self.size.1;
			let scale = self.scale.short();
			format!("{name}-{path}-{format}-{w}-{h}-{scale}")
		}
	}

	fn path(&self) -> String {
		format!(
			"/{name}/{{upload:{UUID_V4_REGEXP}}}{file}",
			name = self.name,
			file = self.file_path
		)
	}

	fn query(&self) -> Option<Vec<(String, String)>> {
		if !self.fallback {
			Some(vec![
				("format".into(), self.format.short().into()),
				(
					"size".into(),
					format!("{w}x{h}", w = self.size.0, h = self.size.1),
				),
				("scale".into(), self.scale.short().into()),
			])
		} else {
			None
		}
	}

	fn path_regex(&self) -> String {
		format!(
			"/media/{name}/({UUID_V4_REGEXP}){file}",
			name = self.name,
			file = self.file_path
		)
	}

	fn path_regex_replacement(&self) -> String {
		let mut filters = Vec::new();
		filters.push("strip_exif()");
		filters.push("strip_icc()");

		// Determine format
		match self.format {
			ResizeFormat::Png => {
				filters.push("format(png)");
			}
			ResizeFormat::Jpeg => {
				filters.push("format(jpeg)");

				// Compression factor
				filters.push("quality(80)");

				// Force PNGs to have a gray background that matches Rivet's
				// theme
				filters.push("background_color(2a2a2a)");
			}
		}

		// Build URL
		//
		// See IMAGOR_UNSAFE in imagor.nomad.tpl
		let mut url = "/unsafe".to_string();
		if matches!(self.scale, ResizeScale::Contain) {
			url.push_str("/fit-in");
		}
		url.push_str(&format!("/{w}x{h}", w = self.size.0, h = self.size.1));
		if !filters.is_empty() {
			url.push_str(&format!("/filters:{}", filters.join(":")));
		}
		url.push_str("/");
		url.push_str(&urlencoding::encode(&format!(
			"http://traffic-server.traffic-server.svc.cluster.local:8080/s3-cache/{ns}-{bucket}/",
			ns = self.ns,
			bucket = self.bucket
		)));
		url.push_str("${1}");
		url.push_str(&urlencoding::encode(&self.file_path));

		url
	}
}

enum ResizeFormat {
	Png,
	Jpeg,
}

impl ResizeFormat {
	fn short(&self) -> &'static str {
		match self {
			ResizeFormat::Png => "png",
			ResizeFormat::Jpeg => "jpeg",
		}
	}
}

enum ResizeScale {
	Cover,
	Contain,
}

impl ResizeScale {
	fn short(&self) -> &'static str {
		match self {
			ResizeScale::Cover => "cover",
			ResizeScale::Contain => "contain",
		}
	}
}

#[derive(serde::Serialize)]
pub struct ResizePresetSerialize {
	key: String,
	path: String,
	query: Option<Vec<(String, String)>>,
	path_regexp: String,
	path_regex_replacement: String,
	game_cors: bool,
	priority: u32,
}

impl From<ResizePreset> for ResizePresetSerialize {
	fn from(preset: ResizePreset) -> ResizePresetSerialize {
		ResizePresetSerialize {
			key: preset.key(),
			path: preset.path(),
			query: preset.query(),
			path_regexp: preset.path_regex(),
			path_regex_replacement: preset.path_regex_replacement(),
			game_cors: preset.game_cors,
			// Match items with query filters before the fallback path
			priority: if preset.fallback { 50 } else { 75 },
		}
	}
}

pub fn build_presets(ns: &str) -> Vec<ResizePreset> {
	// Make sure this matches `hub/src/utils/media-resize.ts`

	let ns = ns.to_string();

	// Determine filters
	let mut presets = Vec::new();
	for src_format in ["png", "jpeg"] {
		// user-avatar & team-avatar
		for name in ["user-avatar", "team-avatar"] {
			presets.push(ResizePreset {
				ns: ns.clone(),
				name: name.into(),
				bucket: format!("bucket-{name}"),
				file_path: format!("/image.{src_format}"),
				format: ResizeFormat::Jpeg,
				size: (128, 128),
				scale: ResizeScale::Cover,
				game_cors: true,
				fallback: true,
			});

			for size in [32, 128, 256, 512] {
				presets.push(ResizePreset {
					ns: ns.clone(),
					name: name.into(),
					bucket: format!("bucket-{name}"),
					file_path: format!("/image.{src_format}"),
					format: ResizeFormat::Jpeg,
					size: (size, size),
					scale: ResizeScale::Cover,
					game_cors: true,
					fallback: false,
				});
			}
		}

		// game-logo
		presets.push(ResizePreset {
			ns: ns.clone(),
			name: "game-logo".into(),
			bucket: "bucket-game-logo".into(),
			file_path: format!("/logo.{src_format}"),
			format: ResizeFormat::Png,
			size: (256, 128),
			scale: ResizeScale::Contain,
			game_cors: true,
			fallback: true,
		});
		for size in [32, 128, 256, 512] {
			presets.push(ResizePreset {
				ns: ns.clone(),
				name: "game-logo".into(),
				bucket: "bucket-game-logo".into(),
				file_path: format!("/logo.{src_format}"),
				format: ResizeFormat::Png,
				size: (size * 2, size),
				scale: ResizeScale::Contain,
				game_cors: true,
				fallback: false,
			});
		}

		// game-banner
		presets.push(ResizePreset {
			ns: ns.clone(),
			name: "game-banner".into(),
			bucket: "bucket-game-banner".into(),
			file_path: format!("/banner.{src_format}"),
			format: ResizeFormat::Jpeg,
			size: (2048, 1024),
			scale: ResizeScale::Cover,
			game_cors: true,
			fallback: true,
		});
		for size in [128, 256, 512, 1024, 2048] {
			presets.push(ResizePreset {
				ns: ns.clone(),
				name: "game-banner".into(),
				bucket: "bucket-game-banner".into(),
				file_path: format!("/banner.{src_format}"),
				format: ResizeFormat::Jpeg,
				size: (size * 2, size),
				scale: ResizeScale::Cover,
				game_cors: true,
				fallback: false,
			});
		}
	}

	presets
}
