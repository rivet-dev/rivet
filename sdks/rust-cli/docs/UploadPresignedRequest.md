# UploadPresignedRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**byte_offset** | **i64** | The byte offset for this multipart chunk. Always 0 if not a multipart upload. | 
**content_length** | **i64** | Expected size of this upload. | 
**path** | **String** | The name of the file to upload. This is the same as the one given in the upload prepare file. | 
**url** | **String** | The URL of the presigned request for which to upload your file to. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


