# ActorActor

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**created_at** | **String** | RFC3339 timestamp | 
**destroyed_at** | Option<**String**> | RFC3339 timestamp | [optional]
**id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**lifecycle** | [**crate::models::ActorLifecycle**](ActorLifecycle.md) |  | 
**network** | [**crate::models::ActorNetwork**](ActorNetwork.md) |  | 
**region** | **String** |  | 
**resources** | [**crate::models::ActorResources**](ActorResources.md) |  | 
**runtime** | [**crate::models::ActorRuntime**](ActorRuntime.md) |  | 
**started_at** | Option<**String**> | RFC3339 timestamp | [optional]
**tags** | Option<[**serde_json::Value**](.md)> |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


