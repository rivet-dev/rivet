# CloudSvcPerf

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**svc_name** | **String** | The name of the service. | 
**ts** | **String** | RFC3339 timestamp | 
**duration** | **i64** | Unsigned 64 bit integer. | 
**req_id** | Option<[**uuid::Uuid**](uuid::Uuid.md)> |  | [optional]
**spans** | [**Vec<crate::models::CloudLogsPerfSpan>**](CloudLogsPerfSpan.md) | A list of performance spans. | 
**marks** | [**Vec<crate::models::CloudLogsPerfMark>**](CloudLogsPerfMark.md) | A list of performance marks. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


