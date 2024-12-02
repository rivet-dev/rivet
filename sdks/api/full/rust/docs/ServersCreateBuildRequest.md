# ServersCreateBuildRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**name** | **String** |  | 
**image_tag** | **String** | A tag given to the game build. | 
**image_file** | [**crate::models::UploadPrepareFile**](UploadPrepareFile.md) |  | 
**multipart_upload** | Option<**bool**> |  | [optional]
**kind** | Option<[**crate::models::ServersBuildKind**](ServersBuildKind.md)> |  | [optional]
**compression** | Option<[**crate::models::ServersBuildCompression**](ServersBuildCompression.md)> |  | [optional]
**prewarm_datacenters** | Option<[**Vec<uuid::Uuid>**](uuid::Uuid.md)> |  | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


