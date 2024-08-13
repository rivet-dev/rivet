# GamesServersServer

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**arguments** | Option<**Vec<String>**> |  | [optional]
**cluster_id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**create_ts** | **i64** |  | 
**datacenter_id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**destroy_ts** | Option<**i64**> |  | [optional]
**environment** | Option<**::std::collections::HashMap<String, String>**> |  | [optional]
**game_id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**image_id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**kill_timeout** | Option<**i64**> | The duration to wait for in milliseconds before killing the server. This should be set to a safe default, and can be overridden during a DELETE request if needed. | [optional]
**network** | [**crate::models::GamesServersNetwork**](GamesServersNetwork.md) |  | 
**resources** | [**crate::models::GamesServersResources**](GamesServersResources.md) |  | 
**server_id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**start_ts** | Option<**i64**> |  | [optional]
**tags** | Option<[**serde_json::Value**](.md)> |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


