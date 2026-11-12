//! Version envelopes for the namespace family: the namespace state machine's state and timer,
//! the commands on a namespace volume, its start notice, and the catalog's listing index.

use crate::generated::pegboard_namespace_v1 as wire;

macro_rules! namespace_v1 {
	($($name:ident),* $(,)?) => {
		$(
			pub enum $name {
				V1(wire::$name),
			}

			impl_versioned_v1!($name, PEGBOARD_NAMESPACE, wire::$name);
		)*
	};
}

namespace_v1!(
	NamespaceState,
	NamespaceTimer,
	NamespaceCreateRequest,
	NamespaceCreateResponse,
	LegacyNamespaceMoveRequest,
	LegacyNamespaceMoveResponse,
	NamespaceReadRequest,
	NamespaceReadResponse,
	NamespaceArchiveRequest,
	NamespaceArchiveResponse,
	NamespaceStepRequest,
	NamespaceStepResponse,
	NamespaceAlarmRequest,
	NamespaceAlarmResponse,
	NamespaceStartNotice,
	NamespaceIndexEntry,
	NamespaceIndexPutRequest,
	NamespaceIndexPutResponse,
	NamespaceIndexListRequest,
	NamespaceIndexListResponse,
	LegacyBackfillNameRecordRequest,
	LegacyBackfillNameRecordResponse,
	LegacyBackfillNameListRequest,
	LegacyBackfillNameListResponse,
);
