# ActorCreateActorRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**region** | Option<**String**> |  | [optional]
**tags** | Option<[**serde_json::Value**](.md)> |  | 
**build** | Option<[**uuid::Uuid**](uuid::Uuid.md)> |  | [optional]
**build_tags** | Option<[**serde_json::Value**](.md)> |  | [optional]
**runtime** | Option<[**crate::models::ActorCreateActorRuntimeRequest**](ActorCreateActorRuntimeRequest.md)> |  | [optional]
**network** | Option<[**crate::models::ActorCreateActorNetworkRequest**](ActorCreateActorNetworkRequest.md)> |  | [optional]
**resources** | Option<[**crate::models::ActorResources**](ActorResources.md)> |  | [optional]
**lifecycle** | Option<[**crate::models::ActorLifecycle**](ActorLifecycle.md)> |  | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


