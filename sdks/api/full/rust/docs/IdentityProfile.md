# IdentityProfile

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**identity_id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**display_name** | **String** | Represent a resource's readable display name. | 
**account_number** | **i32** |  | 
**avatar_url** | **String** | The URL of this identity's avatar image. | 
**is_registered** | **bool** | Whether or not this identity is registered with a linked account. | 
**external** | [**crate::models::IdentityExternalLinks**](IdentityExternalLinks.md) |  | 
**is_admin** | **bool** | Whether or not this identity is an admin. | 
**is_game_linked** | Option<**bool**> | Whether or not this game user has been linked through the Rivet dashboard. | [optional]
**dev_state** | Option<[**crate::models::IdentityDevState**](IdentityDevState.md)> |  | [optional]
**follower_count** | **i64** |  | 
**following_count** | **i64** |  | 
**following** | **bool** | Whether or not the requestee's identity is following this identity. | 
**is_following_me** | **bool** | Whether or not this identity is both following and is followed by the requestee's identity. | 
**is_mutual_following** | **bool** |  | 
**join_ts** | **String** | RFC3339 timestamp | 
**bio** | **String** | Follows regex ^(?:[^\\n\\r]+\\n?|\\n){1,5}$ | 
**linked_accounts** | [**Vec<crate::models::IdentityLinkedAccount>**](IdentityLinkedAccount.md) |  | 
**groups** | [**Vec<crate::models::IdentityGroup>**](IdentityGroup.md) |  | 
**games** | [**Vec<crate::models::GameStatSummary>**](GameStatSummary.md) |  | 
**awaiting_deletion** | Option<**bool**> | Whether or not this identity is awaiting account deletion. Only visible to when the requestee is this identity. | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


