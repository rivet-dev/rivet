# CloudSvcPerf

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**duration** | **i64** | Unsigned 64 bit integer. | 
**marks** | [**Vec<crate::models::CloudLogsPerfMark>**](CloudLogsPerfMark.md) | A list of performance marks. | 
**req_id** | Option<[**uuid::Uuid**](uuid::Uuid.md)> |  | [optional]
**spans** | [**Vec<crate::models::CloudLogsPerfSpan>**](CloudLogsPerfSpan.md) | A list of performance spans. | 
**svc_name** | **String** | The name of the service. | 
**ts** | **String** | RFC3339 timestamp | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


