# CloudVersionCdnCdnConfig

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**build_command** | Option<**String**> | _Configures Rivet CLI behavior. Has no effect on server behavior._ | [optional]
**build_output** | Option<**String**> | _Configures Rivet CLI behavior. Has no effect on server behavior._ | [optional]
**build_env** | Option<**::std::collections::HashMap<String, String>**> | _Configures Rivet CLI behavior. Has no effect on server behavior._ | [optional]
**site_id** | Option<[**uuid::Uuid**](uuid::Uuid.md)> |  | [optional]
**routes** | Option<[**Vec<crate::models::CloudVersionCdnRoute>**](CloudVersionCdnRoute.md)> | Multiple CDN version routes. | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


