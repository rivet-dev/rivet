# ActorPrepareBuildRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**compression** | Option<[**crate::models::ActorBuildCompression**](ActorBuildCompression.md)> |  | [optional]
**image_file** | [**crate::models::UploadPrepareFile**](UploadPrepareFile.md) |  | 
**image_tag** | Option<**String**> | A tag given to the project build. | [optional]
**kind** | Option<[**crate::models::ActorBuildKind**](ActorBuildKind.md)> |  | [optional]
**multipart_upload** | Option<**bool**> |  | [optional]
**name** | **String** |  | 
**prewarm_regions** | Option<**Vec<String>**> |  | [optional]
<<<<<<< HEAD
=======
**tags** | Option<[**serde_json::Value**](.md)> |  | 
>>>>>>> 73a068837 (feat: revamp actor build endpoint, js builds -> tar)

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


