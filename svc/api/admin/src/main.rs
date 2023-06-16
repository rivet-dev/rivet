use api_helper::start;

fn main() {
	start(api_admin::route::handle);
}
