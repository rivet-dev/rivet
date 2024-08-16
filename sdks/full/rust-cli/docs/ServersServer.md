# ServersServer

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**cluster** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**created_at** | **i64** |  | 
**datacenter** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**destroyed_at** | Option<**i64**> |  | [optional]
**environment** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**lifecycle** | [**crate::models::ServersLifecycle**](ServersLifecycle.md) |  | 
**network** | [**crate::models::ServersNetwork**](ServersNetwork.md) |  | 
**resources** | [**crate::models::ServersResources**](ServersResources.md) |  | 
**runtime** | [**crate::models::ServersRuntime**](ServersRuntime.md) |  | 
**started_at** | Option<**i64**> |  | [optional]
**tags** | Option<[**serde_json::Value**](.md)> |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


