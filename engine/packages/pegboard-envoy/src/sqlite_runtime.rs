use rivet_envoy_protocol as protocol;
use sqlite_storage::types::FetchedPage;

pub fn protocol_sqlite_pump_fetched_page(page: FetchedPage) -> protocol::SqliteFetchedPage {
	protocol::SqliteFetchedPage {
		pgno: page.pgno,
		bytes: page.bytes,
	}
}
