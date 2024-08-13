# CloudVersionConfig

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**cdn** | Option<[**crate::models::CloudVersionCdnConfig**](CloudVersionCdnConfig.md)> |  | [optional]
**engine** | Option<[**crate::models::CloudVersionEngineConfig**](CloudVersionEngineConfig.md)> |  | [optional]
**identity** | Option<[**crate::models::CloudVersionIdentityConfig**](CloudVersionIdentityConfig.md)> |  | [optional]
**kv** | Option<[**serde_json::Value**](.md)> | KV configuration for a given version. | [optional]
**matchmaker** | Option<[**crate::models::CloudVersionMatchmakerConfig**](CloudVersionMatchmakerConfig.md)> |  | [optional]
**scripts** | Option<**::std::collections::HashMap<String, String>**> |  | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


