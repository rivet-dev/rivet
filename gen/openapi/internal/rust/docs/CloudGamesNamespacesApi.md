# \CloudGamesNamespacesApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**cloud_games_namespaces_add_namespace_domain**](CloudGamesNamespacesApi.md#cloud_games_namespaces_add_namespace_domain) | **POST** /games/{game_id}/namespaces/{namespace_id}/domains | 
[**cloud_games_namespaces_create_game_namespace**](CloudGamesNamespacesApi.md#cloud_games_namespaces_create_game_namespace) | **POST** /games/{game_id}/namespaces | 
[**cloud_games_namespaces_create_game_namespace_token_development**](CloudGamesNamespacesApi.md#cloud_games_namespaces_create_game_namespace_token_development) | **POST** /games/{game_id}/namespaces/{namespace_id}/tokens/development | 
[**cloud_games_namespaces_create_game_namespace_token_public**](CloudGamesNamespacesApi.md#cloud_games_namespaces_create_game_namespace_token_public) | **POST** /games/{game_id}/namespaces/{namespace_id}/tokens/public | 
[**cloud_games_namespaces_get_game_namespace_by_id**](CloudGamesNamespacesApi.md#cloud_games_namespaces_get_game_namespace_by_id) | **GET** /games/{game_id}/namespaces/{namespace_id} | 
[**cloud_games_namespaces_get_game_namespace_version_history_list**](CloudGamesNamespacesApi.md#cloud_games_namespaces_get_game_namespace_version_history_list) | **GET** /games/{game_id}/namespaces/{namespace_id}/version-history | 
[**cloud_games_namespaces_remove_namespace_cdn_auth_user**](CloudGamesNamespacesApi.md#cloud_games_namespaces_remove_namespace_cdn_auth_user) | **DELETE** /games/{game_id}/namespaces/{namespace_id}/auth-user/{user} | 
[**cloud_games_namespaces_remove_namespace_domain**](CloudGamesNamespacesApi.md#cloud_games_namespaces_remove_namespace_domain) | **DELETE** /games/{game_id}/namespaces/{namespace_id}/domains/{domain} | 
[**cloud_games_namespaces_set_namespace_cdn_auth_type**](CloudGamesNamespacesApi.md#cloud_games_namespaces_set_namespace_cdn_auth_type) | **PUT** /games/{game_id}/namespaces/{namespace_id}/cdn-auth | 
[**cloud_games_namespaces_toggle_namespace_domain_public_auth**](CloudGamesNamespacesApi.md#cloud_games_namespaces_toggle_namespace_domain_public_auth) | **PUT** /games/{game_id}/namespaces/{namespace_id}/domain-public-auth | 
[**cloud_games_namespaces_update_game_namespace_matchmaker_config**](CloudGamesNamespacesApi.md#cloud_games_namespaces_update_game_namespace_matchmaker_config) | **POST** /games/{game_id}/namespaces/{namespace_id}/mm-config | 
[**cloud_games_namespaces_update_game_namespace_version**](CloudGamesNamespacesApi.md#cloud_games_namespaces_update_game_namespace_version) | **PUT** /games/{game_id}/namespaces/{namespace_id}/version | 
[**cloud_games_namespaces_update_namespace_cdn_auth_user**](CloudGamesNamespacesApi.md#cloud_games_namespaces_update_namespace_cdn_auth_user) | **POST** /games/{game_id}/namespaces/{namespace_id}/auth-user | 
[**cloud_games_namespaces_validate_game_namespace**](CloudGamesNamespacesApi.md#cloud_games_namespaces_validate_game_namespace) | **POST** /games/{game_id}/namespaces/validate | 
[**cloud_games_namespaces_validate_game_namespace_matchmaker_config**](CloudGamesNamespacesApi.md#cloud_games_namespaces_validate_game_namespace_matchmaker_config) | **POST** /games/{game_id}/namespaces/{namespace_id}/mm-config/validate | 
[**cloud_games_namespaces_validate_game_namespace_token_development**](CloudGamesNamespacesApi.md#cloud_games_namespaces_validate_game_namespace_token_development) | **POST** /games/{game_id}/namespaces/{namespace_id}/tokens/development/validate | 



## cloud_games_namespaces_add_namespace_domain

> cloud_games_namespaces_add_namespace_domain(game_id, namespace_id, cloud_games_namespaces_add_namespace_domain_request)


Adds a domain to the given game namespace.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**namespace_id** | **uuid::Uuid** |  | [required] |
**cloud_games_namespaces_add_namespace_domain_request** | [**CloudGamesNamespacesAddNamespaceDomainRequest**](CloudGamesNamespacesAddNamespaceDomainRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_namespaces_create_game_namespace

> crate::models::CloudGamesNamespacesCreateGameNamespaceResponse cloud_games_namespaces_create_game_namespace(game_id, cloud_games_namespaces_create_game_namespace_request)


Creates a new namespace for the given game.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**cloud_games_namespaces_create_game_namespace_request** | [**CloudGamesNamespacesCreateGameNamespaceRequest**](CloudGamesNamespacesCreateGameNamespaceRequest.md) |  | [required] |

### Return type

[**crate::models::CloudGamesNamespacesCreateGameNamespaceResponse**](CloudGamesNamespacesCreateGameNamespaceResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_namespaces_create_game_namespace_token_development

> crate::models::CloudGamesNamespacesCreateGameNamespaceTokenDevelopmentResponse cloud_games_namespaces_create_game_namespace_token_development(game_id, namespace_id, cloud_games_namespaces_create_game_namespace_token_development_request)


Creates a development token for the given namespace.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**namespace_id** | **uuid::Uuid** |  | [required] |
**cloud_games_namespaces_create_game_namespace_token_development_request** | [**CloudGamesNamespacesCreateGameNamespaceTokenDevelopmentRequest**](CloudGamesNamespacesCreateGameNamespaceTokenDevelopmentRequest.md) |  | [required] |

### Return type

[**crate::models::CloudGamesNamespacesCreateGameNamespaceTokenDevelopmentResponse**](CloudGamesNamespacesCreateGameNamespaceTokenDevelopmentResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_namespaces_create_game_namespace_token_public

> crate::models::CloudGamesNamespacesCreateGameNamespaceTokenPublicResponse cloud_games_namespaces_create_game_namespace_token_public(game_id, namespace_id)


Creates a public token for the given namespace.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**namespace_id** | **uuid::Uuid** |  | [required] |

### Return type

[**crate::models::CloudGamesNamespacesCreateGameNamespaceTokenPublicResponse**](CloudGamesNamespacesCreateGameNamespaceTokenPublicResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_namespaces_get_game_namespace_by_id

> crate::models::CloudGamesNamespacesGetGameNamespaceByIdResponse cloud_games_namespaces_get_game_namespace_by_id(game_id, namespace_id)


Gets a game namespace by namespace ID.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**namespace_id** | **uuid::Uuid** |  | [required] |

### Return type

[**crate::models::CloudGamesNamespacesGetGameNamespaceByIdResponse**](CloudGamesNamespacesGetGameNamespaceByIdResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_namespaces_get_game_namespace_version_history_list

> crate::models::CloudGamesNamespacesGetGameNamespaceVersionHistoryResponse cloud_games_namespaces_get_game_namespace_version_history_list(game_id, namespace_id, anchor, limit)


Gets the version history for a given namespace.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **String** | A universally unique identifier. | [required] |
**namespace_id** | **String** | A universally unique identifier. | [required] |
**anchor** | Option<**String**> | How many items to offset the search by. |  |
**limit** | Option<**i32**> | Amount of items to return. Must be between 1 and 32 inclusive. |  |

### Return type

[**crate::models::CloudGamesNamespacesGetGameNamespaceVersionHistoryResponse**](CloudGamesNamespacesGetGameNamespaceVersionHistoryResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_namespaces_remove_namespace_cdn_auth_user

> cloud_games_namespaces_remove_namespace_cdn_auth_user(game_id, namespace_id, user)


Removes an authenticated user from the given game namespace.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**namespace_id** | **uuid::Uuid** |  | [required] |
**user** | **String** | A user name. | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_namespaces_remove_namespace_domain

> cloud_games_namespaces_remove_namespace_domain(game_id, namespace_id, domain)


Removes a domain from the given game namespace.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**namespace_id** | **uuid::Uuid** |  | [required] |
**domain** | **String** | A valid domain name (no protocol). | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_namespaces_set_namespace_cdn_auth_type

> cloud_games_namespaces_set_namespace_cdn_auth_type(game_id, namespace_id, cloud_games_namespaces_set_namespace_cdn_auth_type_request)


Updates the CDN authentication type of the given game namesapce.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**namespace_id** | **uuid::Uuid** |  | [required] |
**cloud_games_namespaces_set_namespace_cdn_auth_type_request** | [**CloudGamesNamespacesSetNamespaceCdnAuthTypeRequest**](CloudGamesNamespacesSetNamespaceCdnAuthTypeRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_namespaces_toggle_namespace_domain_public_auth

> cloud_games_namespaces_toggle_namespace_domain_public_auth(game_id, namespace_id, cloud_games_namespaces_toggle_namespace_domain_public_auth_request)


Toggles whether or not to allow authentication based on domain for the given game namesapce.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**namespace_id** | **uuid::Uuid** |  | [required] |
**cloud_games_namespaces_toggle_namespace_domain_public_auth_request** | [**CloudGamesNamespacesToggleNamespaceDomainPublicAuthRequest**](CloudGamesNamespacesToggleNamespaceDomainPublicAuthRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_namespaces_update_game_namespace_matchmaker_config

> cloud_games_namespaces_update_game_namespace_matchmaker_config(game_id, namespace_id, cloud_games_namespaces_update_game_namespace_matchmaker_config_request)


Updates matchmaker config for the given game namespace.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**namespace_id** | **uuid::Uuid** |  | [required] |
**cloud_games_namespaces_update_game_namespace_matchmaker_config_request** | [**CloudGamesNamespacesUpdateGameNamespaceMatchmakerConfigRequest**](CloudGamesNamespacesUpdateGameNamespaceMatchmakerConfigRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_namespaces_update_game_namespace_version

> cloud_games_namespaces_update_game_namespace_version(game_id, namespace_id, cloud_games_namespaces_update_game_namespace_version_request)


Updates the version of a game namespace.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**namespace_id** | **uuid::Uuid** |  | [required] |
**cloud_games_namespaces_update_game_namespace_version_request** | [**CloudGamesNamespacesUpdateGameNamespaceVersionRequest**](CloudGamesNamespacesUpdateGameNamespaceVersionRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_namespaces_update_namespace_cdn_auth_user

> cloud_games_namespaces_update_namespace_cdn_auth_user(game_id, namespace_id, cloud_games_namespaces_update_namespace_cdn_auth_user_request)


Adds an authenticated user to the given game namespace.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**namespace_id** | **uuid::Uuid** |  | [required] |
**cloud_games_namespaces_update_namespace_cdn_auth_user_request** | [**CloudGamesNamespacesUpdateNamespaceCdnAuthUserRequest**](CloudGamesNamespacesUpdateNamespaceCdnAuthUserRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_namespaces_validate_game_namespace

> crate::models::CloudGamesNamespacesValidateGameNamespaceResponse cloud_games_namespaces_validate_game_namespace(game_id, cloud_games_namespaces_validate_game_namespace_request)


Validates information used to create a new game namespace.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**cloud_games_namespaces_validate_game_namespace_request** | [**CloudGamesNamespacesValidateGameNamespaceRequest**](CloudGamesNamespacesValidateGameNamespaceRequest.md) |  | [required] |

### Return type

[**crate::models::CloudGamesNamespacesValidateGameNamespaceResponse**](CloudGamesNamespacesValidateGameNamespaceResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_namespaces_validate_game_namespace_matchmaker_config

> crate::models::CloudGamesNamespacesValidateGameNamespaceMatchmakerConfigResponse cloud_games_namespaces_validate_game_namespace_matchmaker_config(game_id, namespace_id, cloud_games_namespaces_validate_game_namespace_matchmaker_config_request)


Validates information used to update a game namespace's matchmaker config.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**namespace_id** | **uuid::Uuid** |  | [required] |
**cloud_games_namespaces_validate_game_namespace_matchmaker_config_request** | [**CloudGamesNamespacesValidateGameNamespaceMatchmakerConfigRequest**](CloudGamesNamespacesValidateGameNamespaceMatchmakerConfigRequest.md) |  | [required] |

### Return type

[**crate::models::CloudGamesNamespacesValidateGameNamespaceMatchmakerConfigResponse**](CloudGamesNamespacesValidateGameNamespaceMatchmakerConfigResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_namespaces_validate_game_namespace_token_development

> crate::models::CloudGamesNamespacesValidateGameNamespaceTokenDevelopmentResponse cloud_games_namespaces_validate_game_namespace_token_development(game_id, namespace_id, cloud_games_namespaces_validate_game_namespace_token_development_request)


Validates information used to create a new game namespace development token.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**namespace_id** | **uuid::Uuid** |  | [required] |
**cloud_games_namespaces_validate_game_namespace_token_development_request** | [**CloudGamesNamespacesValidateGameNamespaceTokenDevelopmentRequest**](CloudGamesNamespacesValidateGameNamespaceTokenDevelopmentRequest.md) |  | [required] |

### Return type

[**crate::models::CloudGamesNamespacesValidateGameNamespaceTokenDevelopmentResponse**](CloudGamesNamespacesValidateGameNamespaceTokenDevelopmentResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

