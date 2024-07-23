# ServersCreateServerRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**arguments** | Option<**Vec<String>**> |  | [optional]
**datacenter** | **String** | The name ID of the datacenter | 
**environment** | Option<**::std::collections::HashMap<String, String>**> |  | [optional]
**image_id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**kill_timeout** | Option<**i64**> | The duration to wait for in milliseconds before killing the server. This should be set to a safe default, and can be overridden during a DELETE request if needed. | [optional]
**network** | [**crate::models::ServersCreateServerNetworkRequest**](ServersCreateServerNetworkRequest.md) |  | 
**resources** | [**crate::models::ServersResources**](ServersResources.md) |  | 
**tags** | Option<[**serde_json::Value**](.md)> |  | 
**webhook_url** | Option<**String**> | A url to send to which events from the server running will be sent | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


