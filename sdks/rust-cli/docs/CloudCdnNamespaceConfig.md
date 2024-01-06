# CloudCdnNamespaceConfig

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**auth_type** | [**crate::models::CloudCdnAuthType**](CloudCdnAuthType.md) |  | 
**auth_user_list** | [**Vec<crate::models::CloudCdnNamespaceAuthUser>**](CloudCdnNamespaceAuthUser.md) | A list of CDN authenticated users for a given namespace. | 
**domains** | [**Vec<crate::models::CloudCdnNamespaceDomain>**](CloudCdnNamespaceDomain.md) | A list of CDN domains for a given namespace. | 
**enable_domain_public_auth** | **bool** | Whether or not to allow users to connect to the given namespace via domain name. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


