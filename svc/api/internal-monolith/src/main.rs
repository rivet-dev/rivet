use api_helper::start;

fn main() {
	start(api_internal_monolith::route::handle);
}
