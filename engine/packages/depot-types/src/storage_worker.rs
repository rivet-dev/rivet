//! Conversions between single storage worker messages and the items of a coalesced batch.

use rivet_data::generated::depot_storage_worker_protocol_v1::{
	StorageWorkerBatchItem as Item, StorageWorkerBatchItemResponse as ItemResponse,
	StorageWorkerBatchRequest, StorageWorkerRequest as Request, StorageWorkerResponse as Response,
};

/// Moves a single request into a batch item. A batch cannot nest, so a batch request is
/// returned unchanged as the error.
pub fn batch_item(request: Request) -> Result<Item, StorageWorkerBatchRequest> {
	Ok(match request {
		Request::GetDiskIncarnationRequest => Item::GetDiskIncarnationRequest,
		Request::ClaimRequest(r) => Item::ClaimRequest(r),
		Request::AppendRequest(r) => Item::AppendRequest(r),
		Request::AdvanceCommittedRequest(r) => Item::AdvanceCommittedRequest(r),
		Request::TruncateRequest(r) => Item::TruncateRequest(r),
		Request::ReadVolumeLogEntriesRequest(r) => Item::ReadVolumeLogEntriesRequest(r),
		Request::MarkInSyncRequest(r) => Item::MarkInSyncRequest(r),
		Request::LoadVolumeRequest(r) => Item::LoadVolumeRequest(r),
		Request::MarkVolumeDeletedRequest(r) => Item::MarkVolumeDeletedRequest(r),
		Request::TrimRequest(r) => Item::TrimRequest(r),
		Request::ForgetVolumeRequest(r) => Item::ForgetVolumeRequest(r),
		Request::ListTabletReplicasRequest(r) => Item::ListTabletReplicasRequest(r),
		Request::ReadVolumeLogStatusRequest(r) => Item::ReadVolumeLogStatusRequest(r),
		Request::ForgetVolumesRequest(r) => Item::ForgetVolumesRequest(r),
		Request::TrimVolumesRequest(r) => Item::TrimVolumesRequest(r),
		Request::StorageWorkerBatchRequest(batch) => return Err(batch),
	})
}

/// Moves a batch item back into the request it was coalesced from.
pub fn item_request(item: Item) -> Request {
	match item {
		Item::GetDiskIncarnationRequest => Request::GetDiskIncarnationRequest,
		Item::ClaimRequest(r) => Request::ClaimRequest(r),
		Item::AppendRequest(r) => Request::AppendRequest(r),
		Item::AdvanceCommittedRequest(r) => Request::AdvanceCommittedRequest(r),
		Item::TruncateRequest(r) => Request::TruncateRequest(r),
		Item::ReadVolumeLogEntriesRequest(r) => Request::ReadVolumeLogEntriesRequest(r),
		Item::MarkInSyncRequest(r) => Request::MarkInSyncRequest(r),
		Item::LoadVolumeRequest(r) => Request::LoadVolumeRequest(r),
		Item::MarkVolumeDeletedRequest(r) => Request::MarkVolumeDeletedRequest(r),
		Item::TrimRequest(r) => Request::TrimRequest(r),
		Item::ForgetVolumeRequest(r) => Request::ForgetVolumeRequest(r),
		Item::ListTabletReplicasRequest(r) => Request::ListTabletReplicasRequest(r),
		Item::ReadVolumeLogStatusRequest(r) => Request::ReadVolumeLogStatusRequest(r),
		Item::ForgetVolumesRequest(r) => Request::ForgetVolumesRequest(r),
		Item::TrimVolumesRequest(r) => Request::TrimVolumesRequest(r),
	}
}

/// Moves a single response into a batch item response. A batch response answers a batch,
/// never one item, so it is returned unchanged as the error.
pub fn item_response(response: Response) -> Result<ItemResponse, Response> {
	Ok(match response {
		Response::GetDiskIncarnationResponse(r) => ItemResponse::GetDiskIncarnationResponse(r),
		Response::ReplicaStateResponse(r) => ItemResponse::ReplicaStateResponse(r),
		Response::ReadVolumeLogEntriesResponse(r) => ItemResponse::ReadVolumeLogEntriesResponse(r),
		Response::ListTabletReplicasResponse(r) => ItemResponse::ListTabletReplicasResponse(r),
		Response::RejectResponse(r) => ItemResponse::RejectResponse(r),
		Response::ReadVolumeLogStatusResponse(r) => ItemResponse::ReadVolumeLogStatusResponse(r),
		Response::ForgetVolumesResponse(r) => ItemResponse::ForgetVolumesResponse(r),
		Response::TrimVolumesResponse(r) => ItemResponse::TrimVolumesResponse(r),
		Response::StorageWorkerBatchResponse(_) => return Err(response),
	})
}

/// Moves a batch item response into the response a single request would have received.
pub fn response_from_item(item: ItemResponse) -> Response {
	match item {
		ItemResponse::GetDiskIncarnationResponse(r) => Response::GetDiskIncarnationResponse(r),
		ItemResponse::ReplicaStateResponse(r) => Response::ReplicaStateResponse(r),
		ItemResponse::ReadVolumeLogEntriesResponse(r) => Response::ReadVolumeLogEntriesResponse(r),
		ItemResponse::ListTabletReplicasResponse(r) => Response::ListTabletReplicasResponse(r),
		ItemResponse::RejectResponse(r) => Response::RejectResponse(r),
		ItemResponse::ReadVolumeLogStatusResponse(r) => Response::ReadVolumeLogStatusResponse(r),
		ItemResponse::ForgetVolumesResponse(r) => Response::ForgetVolumesResponse(r),
		ItemResponse::TrimVolumesResponse(r) => Response::TrimVolumesResponse(r),
	}
}

/// Bounded metric label for a request; a coalesced batch is `batch`.
pub fn request_label(request: &Request) -> &'static str {
	match request {
		Request::GetDiskIncarnationRequest => "get_disk_incarnation",
		Request::ClaimRequest(_) => "claim",
		Request::AppendRequest(_) => "append",
		Request::AdvanceCommittedRequest(_) => "advance_committed",
		Request::TruncateRequest(_) => "truncate",
		Request::ReadVolumeLogEntriesRequest(_) => "read",
		Request::MarkInSyncRequest(_) => "mark_in_sync",
		Request::LoadVolumeRequest(_) => "load_volume",
		Request::MarkVolumeDeletedRequest(_) => "mark_volume_deleted",
		Request::TrimRequest(_) => "trim",
		Request::ForgetVolumeRequest(_) => "forget_volume",
		Request::ListTabletReplicasRequest(_) => "list_tablet_replicas",
		Request::ReadVolumeLogStatusRequest(_) => "read_status",
		Request::ForgetVolumesRequest(_) => "forget_volumes",
		Request::TrimVolumesRequest(_) => "trim_volumes",
		Request::StorageWorkerBatchRequest(_) => "batch",
	}
}

/// Bounded metric label for a batch item, the same label its request has alone.
pub fn item_label(item: &Item) -> &'static str {
	match item {
		Item::GetDiskIncarnationRequest => "get_disk_incarnation",
		Item::ClaimRequest(_) => "claim",
		Item::AppendRequest(_) => "append",
		Item::AdvanceCommittedRequest(_) => "advance_committed",
		Item::TruncateRequest(_) => "truncate",
		Item::ReadVolumeLogEntriesRequest(_) => "read",
		Item::MarkInSyncRequest(_) => "mark_in_sync",
		Item::LoadVolumeRequest(_) => "load_volume",
		Item::MarkVolumeDeletedRequest(_) => "mark_volume_deleted",
		Item::TrimRequest(_) => "trim",
		Item::ForgetVolumeRequest(_) => "forget_volume",
		Item::ListTabletReplicasRequest(_) => "list_tablet_replicas",
		Item::ReadVolumeLogStatusRequest(_) => "read_status",
		Item::ForgetVolumesRequest(_) => "forget_volumes",
		Item::TrimVolumesRequest(_) => "trim_volumes",
	}
}
