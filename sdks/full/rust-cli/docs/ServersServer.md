# ServersServer

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**arguments** | Option<**Vec<String>**> |  | [optional]
**cluster** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**created_at** | **i64** |  | 
**datacenter** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**destroyed_at** | Option<**i64**> |  | [optional]
**environment** | Option<**::std::collections::HashMap<String, String>**> |  | [optional]
**game** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**image** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**kill_timeout** | Option<**i64**> | The duration to wait for in milliseconds before killing the server. This should be set to a safe default, and can be overridden during a DELETE request if needed. | [optional]
**network** | [**crate::models::ServersNetwork**](ServersNetwork.md) |  | 
**resources** | [**crate::models::ServersResources**](ServersResources.md) |  | 
**started_at** | Option<**i64**> |  | [optional]
**tags** | Option<[**serde_json::Value**](.md)> |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


