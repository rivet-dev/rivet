# ActorsGetActorLogsResponse

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**actor_ids** | **Vec<String>** | List of actor IDs in these logs. The order of these correspond to the index in the log entry. | 
**lines** | **Vec<String>** | Sorted old to new. | 
**timestamps** | **Vec<String>** | Sorted old to new. | 
**streams** | **Vec<i32>** | Streams the logs came from.  0 = stdout 1 = stderr | 
**actor_indices** | **Vec<i32>** | Index of the actor that this log was for. Use this index to look the full ID in `actor_ids`. | 
**watch** | [**crate::models::WatchResponse**](WatchResponse.md) |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


