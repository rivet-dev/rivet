# IdentitySetupResponse

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**game_id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**identity** | [**crate::models::IdentityProfile**](IdentityProfile.md) |  | 
**identity_token** | **String** | Documentation at https://jwt.io/ | 
**identity_token_expire_ts** | **String** | If this token is compromised, anyone with access to this token has control of the identity.  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


