use depot::types::FetchedPage;
use rivet_envoy_protocol as protocol;

pub fn protocol_sqlite_conveyer_fetched_page(page: FetchedPage) -> protocol::SqliteFetchedPage {
	protocol::SqliteFetchedPage {
		pgno: page.pgno,
		bytes: page.bytes,
	}
}
