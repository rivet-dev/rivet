# \ActorApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**actor_create**](ActorApi.md#actor_create) | **POST** /actors | 
[**actor_destroy**](ActorApi.md#actor_destroy) | **DELETE** /actors/{actor} | 
[**actor_get**](ActorApi.md#actor_get) | **GET** /actors/{actor} | 
[**actor_list**](ActorApi.md#actor_list) | **GET** /actors | 
[**actor_upgrade**](ActorApi.md#actor_upgrade) | **POST** /actors/{actor}/upgrade | 
[**actor_upgrade_all**](ActorApi.md#actor_upgrade_all) | **POST** /actors/upgrade | 



## actor_create

> crate::models::ActorCreateActorResponse actor_create(actor_create_actor_request, project, environment, endpoint_type)


Create a new dynamic actor.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**actor_create_actor_request** | [**ActorCreateActorRequest**](ActorCreateActorRequest.md) |  | [required] |
**project** | Option<**String**> |  |  |
**environment** | Option<**String**> |  |  |
**endpoint_type** | Option<[**ActorEndpointType**](.md)> |  |  |

### Return type

[**crate::models::ActorCreateActorResponse**](ActorCreateActorResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## actor_destroy

> serde_json::Value actor_destroy(actor, project, environment, override_kill_timeout)


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


## actor_get

> crate::models::ActorGetActorResponse actor_get(actor, project, environment, endpoint_type)


Gets a dynamic actor.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**actor** | **uuid::Uuid** | The id of the actor to destroy | [required] |
**project** | Option<**String**> |  |  |
**environment** | Option<**String**> |  |  |
**endpoint_type** | Option<[**ActorEndpointType**](.md)> |  |  |

### Return type

[**crate::models::ActorGetActorResponse**](ActorGetActorResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## actor_list

> crate::models::ActorListActorsResponse actor_list(project, environment, endpoint_type, tags_json, include_destroyed, cursor)


Lists all actors associated with the token used. Can be filtered by tags in the query string.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**project** | Option<**String**> |  |  |
**environment** | Option<**String**> |  |  |
**endpoint_type** | Option<[**ActorEndpointType**](.md)> |  |  |
**tags_json** | Option<**String**> |  |  |
**include_destroyed** | Option<**bool**> |  |  |
**cursor** | Option<**String**> |  |  |

### Return type

[**crate::models::ActorListActorsResponse**](ActorListActorsResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## actor_upgrade

> serde_json::Value actor_upgrade(actor, actor_upgrade_actor_request, project, environment)


Upgrades a dynamic actor.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**actor** | **uuid::Uuid** | The id of the actor to upgrade | [required] |
**actor_upgrade_actor_request** | [**ActorUpgradeActorRequest**](ActorUpgradeActorRequest.md) |  | [required] |
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


## actor_upgrade_all

> crate::models::ActorUpgradeAllActorsResponse actor_upgrade_all(actor_upgrade_all_actors_request, project, environment)


Upgrades a dynamic actor.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**actor_upgrade_all_actors_request** | [**ActorUpgradeAllActorsRequest**](ActorUpgradeAllActorsRequest.md) |  | [required] |
**project** | Option<**String**> |  |  |
**environment** | Option<**String**> |  |  |

### Return type

[**crate::models::ActorUpgradeAllActorsResponse**](ActorUpgradeAllActorsResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

