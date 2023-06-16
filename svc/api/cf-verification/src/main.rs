use api_helper::start;

fn main() {
	start(api_cf_verification::route::handle);
}
