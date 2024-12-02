# ServersServer

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**environment** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**datacenter** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**tags** | Option<[**serde_json::Value**](.md)> |  | 
**runtime** | [**crate::models::ServersRuntime**](ServersRuntime.md) |  | 
**network** | [**crate::models::ServersNetwork**](ServersNetwork.md) |  | 
**resources** | [**crate::models::ServersResources**](ServersResources.md) |  | 
**lifecycle** | [**crate::models::ServersLifecycle**](ServersLifecycle.md) |  | 
**created_at** | **i64** |  | 
**started_at** | Option<**i64**> |  | [optional]
**destroyed_at** | Option<**i64**> |  | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


