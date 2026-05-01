use rivet_envoy_protocol as protocol;
use depot::types::FetchedPage;

pub fn protocol_sqlite_conveyer_fetched_page(page: FetchedPage) -> protocol::SqliteFetchedPage {
	protocol::SqliteFetchedPage {
		pgno: page.pgno,
		bytes: page.bytes,
	}
}
