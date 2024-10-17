# IdentityProfile

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**account_number** | **i32** |  | 
**avatar_url** | **String** | The URL of this identity's avatar image. | 
**awaiting_deletion** | Option<**bool**> | Whether or not this identity is awaiting account deletion. Only visible to when the requestee is this identity. | [optional]
**bio** | **String** | Follows regex ^(?:[^\\n\\r]+\\n?|\\n){1,5}$ | 
**dev_state** | Option<[**crate::models::IdentityDevState**](IdentityDevState.md)> |  | [optional]
**display_name** | **String** | Represent a resource's readable display name. | 
**external** | [**crate::models::IdentityExternalLinks**](IdentityExternalLinks.md) |  | 
**follower_count** | **i64** |  | 
**following** | **bool** | Whether or not the requestee's identity is following this identity. | 
**following_count** | **i64** |  | 
**games** | [**Vec<crate::models::GameStatSummary>**](GameStatSummary.md) |  | 
**groups** | [**Vec<crate::models::IdentityGroup>**](IdentityGroup.md) |  | 
**identity_id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**is_admin** | **bool** | Whether or not this identity is an admin. | 
**is_following_me** | **bool** | Whether or not this identity is both following and is followed by the requestee's identity. | 
**is_game_linked** | Option<**bool**> | Whether or not this game user has been linked through the Rivet dashboard. | [optional]
**is_mutual_following** | **bool** |  | 
**is_registered** | **bool** | Whether or not this identity is registered with a linked account. | 
**join_ts** | **String** | RFC3339 timestamp | 
**linked_accounts** | [**Vec<crate::models::IdentityLinkedAccount>**](IdentityLinkedAccount.md) |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


