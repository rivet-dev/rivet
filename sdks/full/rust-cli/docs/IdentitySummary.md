# IdentitySummary

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**account_number** | **i32** |  | 
**avatar_url** | **String** | The URL of this identity's avatar image. | 
**display_name** | **String** | Represent a resource's readable display name. | 
**external** | [**crate::models::IdentityExternalLinks**](IdentityExternalLinks.md) |  | 
**following** | **bool** | Whether or not the requestee's identity is following this identity. | 
**identity_id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**is_following_me** | **bool** | Whether or not this identity is both following and is followed by the requestee's identity. | 
**is_mutual_following** | **bool** |  | 
**is_registered** | **bool** | Whether or not this identity is registered with a linked account. | 
**presence** | Option<[**crate::models::IdentityPresence**](IdentityPresence.md)> |  | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


