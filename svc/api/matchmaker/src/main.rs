use api_helper::start;

fn main() {
	start(api_matchmaker::route::handle);
}
