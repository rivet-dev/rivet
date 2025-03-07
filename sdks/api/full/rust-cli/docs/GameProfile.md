# GameProfile

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**banner_url** | Option<**String**> | The URL of this game's banner image. | [optional]
**description** | **String** | A description of the given game. | 
**developer** | [**crate::models::GroupSummary**](GroupSummary.md) |  | 
**display_name** | **String** | Represent a resource's readable display name. | 
**game_id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**group_leaderboard_categories** | [**Vec<crate::models::GameLeaderboardCategory>**](GameLeaderboardCategory.md) | A list of game leaderboard categories. | 
**identity_leaderboard_categories** | [**Vec<crate::models::GameLeaderboardCategory>**](GameLeaderboardCategory.md) | A list of game leaderboard categories. | 
**logo_url** | Option<**String**> | The URL of this game's logo image. | [optional]
**name_id** | **String** | A human readable short identifier used to references resources. Different than a `rivet.common#Uuid` because this is intended to be human readable. Different than `rivet.common#DisplayName` because this should not include special characters and be short. | 
**platforms** | [**Vec<crate::models::GamePlatformLink>**](GamePlatformLink.md) | A list of platform links. | 
**recommended_groups** | [**Vec<crate::models::GroupSummary>**](GroupSummary.md) | A list of group summaries. | 
**tags** | **Vec<String>** | A list of game tags. | 
**url** | **String** | The URL to this game's website. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


