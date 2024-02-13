# KvPutEntry

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**key** | **String** | A string representing a key in the key-value database. Maximum length of 512 characters. _Recommended Key Path Format_ Key path components are split by a slash (e.g. `a/b/c` has the path components `[\"a\", \"b\", \"c\"]`). Slashes can be escaped by using a backslash (e.g. `a/b/c/d` has the path components `[\"a\", \"b/c\", \"d\"]`). This format is not enforced by Rivet, but the tools built around Rivet KV work better if this format is used. | 
**value** | Option<[**serde_json::Value**](.md)> | A JSON object stored in the KV database. A `null` value indicates the entry is deleted. Maximum length of 262,144 bytes when encoded. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


