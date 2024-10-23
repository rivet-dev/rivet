# ActorCreateActorRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**datacenter** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**lifecycle** | Option<[**crate::models::ActorLifecycle**](ActorLifecycle.md)> |  | [optional]
**network** | [**crate::models::ActorCreateActorNetworkRequest**](ActorCreateActorNetworkRequest.md) |  | 
**resources** | [**crate::models::ActorResources**](ActorResources.md) |  | 
**runtime** | [**crate::models::ActorCreateActorRuntimeRequest**](ActorCreateActorRuntimeRequest.md) |  | 
**tags** | Option<[**serde_json::Value**](.md)> |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


