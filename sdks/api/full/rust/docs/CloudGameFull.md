# CloudGameFull

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**available_regions** | [**Vec<crate::models::CloudRegionSummary>**](CloudRegionSummary.md) | A list of region summaries. | 
**banner_url** | Option<**String**> | The URL of this game's banner image. | [optional]
**create_ts** | **String** | RFC3339 timestamp | 
**developer_group_id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**display_name** | **String** | Represent a resource's readable display name. | 
**game_id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**logo_url** | Option<**String**> | The URL of this game's logo image. | [optional]
**name_id** | **String** | A human readable short identifier used to references resources. Different than a `rivet.common#Uuid` because this is intended to be human readable. Different than `rivet.common#DisplayName` because this should not include special characters and be short. | 
**namespaces** | [**Vec<crate::models::CloudNamespaceSummary>**](CloudNamespaceSummary.md) | A list of namespace summaries. | 
**total_player_count** | **i32** | Unsigned 32 bit integer. | 
**versions** | [**Vec<crate::models::CloudVersionSummary>**](CloudVersionSummary.md) | A list of version summaries. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


