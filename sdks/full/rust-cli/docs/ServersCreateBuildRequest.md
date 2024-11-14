# ServersCreateBuildRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**compression** | Option<[**crate::models::ServersBuildCompression**](ServersBuildCompression.md)> |  | [optional]
**image_file** | [**crate::models::UploadPrepareFile**](UploadPrepareFile.md) |  | 
**image_tag** | **String** | A tag given to the game build. | 
**kind** | Option<[**crate::models::ServersBuildKind**](ServersBuildKind.md)> |  | [optional]
**multipart_upload** | Option<**bool**> |  | [optional]
**name** | **String** |  | 
**prewarm_datacenters** | Option<[**Vec<uuid::Uuid>**](uuid::Uuid.md)> |  | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


