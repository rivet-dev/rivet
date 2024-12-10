# GameGameSummary

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**game_id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**name_id** | **String** | A human readable short identifier used to references resources. Different than a `uuid` because this is intended to be human readable. Different than `DisplayName` because this should not include special characters and be short. | 
**display_name** | **String** | Represent a resource's readable display name. | 
**logo_url** | Option<**String**> | The URL of this game's logo image. | [optional]
**banner_url** | Option<**String**> | The URL of this game's banner image. | [optional]
**url** | **String** |  | 
**developer** | [**crate::models::GroupHandle**](GroupHandle.md) |  | 
**total_player_count** | **i32** | Unsigned 32 bit integer. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


