# ActorsActor

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**region** | **String** |  | 
**tags** | Option<[**serde_json::Value**](.md)> |  | 
**runtime** | [**crate::models::ActorsRuntime**](ActorsRuntime.md) |  | 
**network** | [**crate::models::ActorsNetwork**](ActorsNetwork.md) |  | 
**resources** | [**crate::models::ActorsResources**](ActorsResources.md) |  | 
**lifecycle** | [**crate::models::ActorsLifecycle**](ActorsLifecycle.md) |  | 
**created_at** | **String** | RFC3339 timestamp | 
**started_at** | Option<**String**> | RFC3339 timestamp | [optional]
**destroyed_at** | Option<**String**> | RFC3339 timestamp | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


