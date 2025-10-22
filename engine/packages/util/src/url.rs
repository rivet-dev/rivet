pub fn to_string_without_slash(url: &url::Url) -> String {
	url.to_string().trim_end_matches('/').to_string()
}
