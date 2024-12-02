# IdentitySummary

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**identity_id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**display_name** | **String** | Represent a resource's readable display name. | 
**account_number** | **i32** |  | 
**avatar_url** | **String** | The URL of this identity's avatar image. | 
**is_registered** | **bool** | Whether or not this identity is registered with a linked account. | 
**external** | [**crate::models::IdentityExternalLinks**](IdentityExternalLinks.md) |  | 
**following** | **bool** | Whether or not the requestee's identity is following this identity. | 
**is_following_me** | **bool** | Whether or not this identity is both following and is followed by the requestee's identity. | 
**is_mutual_following** | **bool** |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


