# ServersCreateBuildRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**compression** | Option<[**crate::models::ServersBuildCompression**](ServersBuildCompression.md)> |  | [optional]
**display_name** | **String** | Represent a resource's readable display name. | 
**image_file** | [**crate::models::UploadPrepareFile**](UploadPrepareFile.md) |  | 
**image_tag** | **String** | A tag given to the game build. | 
**kind** | Option<[**crate::models::ServersBuildKind**](ServersBuildKind.md)> |  | [optional]
**multipart_upload** | Option<**bool**> |  | [optional]
**tags** | Option<[**serde_json::Value**](.md)> |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


