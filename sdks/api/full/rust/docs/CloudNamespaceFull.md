# CloudNamespaceFull

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**namespace_id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**create_ts** | **String** | RFC3339 timestamp | 
**display_name** | **String** | Represent a resource's readable display name. | 
**version_id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**name_id** | **String** | A human readable short identifier used to references resources. Different than a `rivet.common#Uuid` because this is intended to be human readable. Different than `rivet.common#DisplayName` because this should not include special characters and be short. | 
**config** | [**crate::models::CloudNamespaceConfig**](CloudNamespaceConfig.md) |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


