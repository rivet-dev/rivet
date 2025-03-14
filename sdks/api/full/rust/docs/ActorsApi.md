# \ActorsApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**actors_create**](ActorsApi.md#actors_create) | **POST** /actors | 
[**actors_destroy**](ActorsApi.md#actors_destroy) | **DELETE** /actors/{actor} | 
[**actors_get**](ActorsApi.md#actors_get) | **GET** /actors/{actor} | 
[**actors_list**](ActorsApi.md#actors_list) | **GET** /actors | 
[**actors_upgrade**](ActorsApi.md#actors_upgrade) | **POST** /actors/{actor}/upgrade | 
[**actors_upgrade_all**](ActorsApi.md#actors_upgrade_all) | **POST** /actors/upgrade | 



## actors_create

> crate::models::ActorsCreateActorResponse actors_create(actors_create_actor_request, project, environment, endpoint_type)


Create a new dynamic actor.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**actors_create_actor_request** | [**ActorsCreateActorRequest**](ActorsCreateActorRequest.md) |  | [required] |
**project** | Option<**String**> |  |  |
**environment** | Option<**String**> |  |  |
**endpoint_type** | Option<[**ActorsEndpointType**](.md)> |  |  |

### Return type

[**crate::models::ActorsCreateActorResponse**](ActorsCreateActorResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## actors_destroy

> serde_json::Value actors_destroy(actor, project, environment, override_kill_timeout)


Destroy a dynamic actor.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**actor** | **uuid::Uuid** | The id of the actor to destroy | [required] |
**project** | Option<**String**> |  |  |
**environment** | Option<**String**> |  |  |
**override_kill_timeout** | Option<**i64**> | The duration to wait for in milliseconds before killing the actor. This should be used to override the default kill timeout if a faster time is needed, say for ignoring a graceful shutdown. |  |

### Return type

[**serde_json::Value**](serde_json::Value.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## actors_get

> crate::models::ActorsGetActorResponse actors_get(actor, project, environment, endpoint_type)


Gets a dynamic actor.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**actor** | **uuid::Uuid** | The id of the actor to destroy | [required] |
**project** | Option<**String**> |  |  |
**environment** | Option<**String**> |  |  |
**endpoint_type** | Option<[**ActorsEndpointType**](.md)> |  |  |

### Return type

[**crate::models::ActorsGetActorResponse**](ActorsGetActorResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## actors_list

> crate::models::ActorsListActorsResponse actors_list(project, environment, endpoint_type, tags_json, include_destroyed, cursor)


Lists all actors associated with the token used. Can be filtered by tags in the query string.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**project** | Option<**String**> |  |  |
**environment** | Option<**String**> |  |  |
**endpoint_type** | Option<[**ActorsEndpointType**](.md)> |  |  |
**tags_json** | Option<**String**> |  |  |
**include_destroyed** | Option<**bool**> |  |  |
**cursor** | Option<**String**> |  |  |

### Return type

[**crate::models::ActorsListActorsResponse**](ActorsListActorsResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## actors_upgrade

> serde_json::Value actors_upgrade(actor, actors_upgrade_actor_request, project, environment)


Upgrades a dynamic actor.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**actor** | **uuid::Uuid** | The id of the actor to upgrade | [required] |
**actors_upgrade_actor_request** | [**ActorsUpgradeActorRequest**](ActorsUpgradeActorRequest.md) |  | [required] |
**project** | Option<**String**> |  |  |
**environment** | Option<**String**> |  |  |

### Return type

[**serde_json::Value**](serde_json::Value.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## actors_upgrade_all

> crate::models::ActorsUpgradeAllActorsResponse actors_upgrade_all(actors_upgrade_all_actors_request, project, environment)


Upgrades a dynamic actor.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**actors_upgrade_all_actors_request** | [**ActorsUpgradeAllActorsRequest**](ActorsUpgradeAllActorsRequest.md) |  | [required] |
**project** | Option<**String**> |  |  |
**environment** | Option<**String**> |  |  |

### Return type

[**crate::models::ActorsUpgradeAllActorsResponse**](ActorsUpgradeAllActorsResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

