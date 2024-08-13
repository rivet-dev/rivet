# KvGetResponse

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**deleted** | Option<**bool**> | Whether or not the entry has been deleted. Only set when watching this endpoint. | [optional]
**value** | Option<[**serde_json::Value**](.md)> | A JSON object stored in the KV database. A `null` value indicates the entry is deleted. Maximum length of 262,144 bytes when encoded. | 
**watch** | [**crate::models::WatchResponse**](WatchResponse.md) |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


