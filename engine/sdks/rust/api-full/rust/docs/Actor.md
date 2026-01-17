# Actor

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**actor_id** | **String** |  | 
**connectable_ts** | Option<**i64**> | Denotes when the actor was last connectable. Null if actor is not running. | [optional]
**crash_policy** | [**models::CrashPolicy**](CrashPolicy.md) |  | 
**create_ts** | **i64** | Denotes when the actor was first created. | 
**datacenter** | **String** |  | 
**destroy_ts** | Option<**i64**> | Denotes when the actor was destroyed. | [optional]
**error** | Option<[**serde_json::Value**](.md)> | Error details if the actor failed to start. | [optional]
**key** | Option<**String**> |  | [optional]
**name** | **String** |  | 
**namespace_id** | **String** |  | 
**pending_allocation_ts** | Option<**i64**> | Denotes when the actor started waiting for an allocation. | [optional]
**reschedule_ts** | Option<**i64**> | Denotes when the actor will try to allocate again. If this is set, the actor will not attempt to allocate until the given timestamp. | [optional]
**runner_name_selector** | **String** |  | 
**sleep_ts** | Option<**i64**> | Denotes when the actor entered a sleeping state. | [optional]
**start_ts** | Option<**i64**> | Denotes when the actor was first made connectable. Null if never. | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


