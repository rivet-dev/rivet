# \ActorsApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**actors_create**](ActorsApi.md#actors_create) | **POST** /actors | 
[**actors_destroy**](ActorsApi.md#actors_destroy) | **DELETE** /actors/{actor} | 
[**actors_get**](ActorsApi.md#actors_get) | **GET** /actors/{actor} | 
[**actors_list**](ActorsApi.md#actors_list) | **GET** /actors | 
[**actors_query**](ActorsApi.md#actors_query) | **GET** /actors/query | 
[**actors_upgrade**](ActorsApi.md#actors_upgrade) | **POST** /actors/{actor}/upgrade | 
[**actors_upgrade_all**](ActorsApi.md#actors_upgrade_all) | **POST** /actors/upgrade | 
[**actors_usage**](ActorsApi.md#actors_usage) | **GET** /actors/usage | 



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


## actors_query

> crate::models::ActorsQueryActorsResponse actors_query(query_json, project, environment, cursor)


Queries actors using a JSON-encoded query expression. Supports pagination with cursor-based navigation.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**query_json** | **String** | JSON-encoded query expression for filtering actors | [required] |
**project** | Option<**String**> |  |  |
**environment** | Option<**String**> |  |  |
**cursor** | Option<**String**> | Cursor for pagination |  |

### Return type

[**crate::models::ActorsQueryActorsResponse**](ActorsQueryActorsResponse.md)

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


## actors_usage

> crate::models::ActorsGetActorUsageResponse actors_usage(start, end, interval, project, environment, group_by, query_json)


Returns time series data for actor usage metrics. Allows filtering and grouping by various actor properties.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**start** | **i32** | Start timestamp in milliseconds | [required] |
**end** | **i32** | End timestamp in milliseconds | [required] |
**interval** | **i32** | Time bucket interval in milliseconds | [required] |
**project** | Option<**String**> |  |  |
**environment** | Option<**String**> |  |  |
**group_by** | Option<**String**> | JSON-encoded KeyPath for grouping results (e.g. {\"property\":\"datacenter_id\"} or {\"property\":\"tags\",\"map_key\":\"region\"}) |  |
**query_json** | Option<**String**> | JSON-encoded query expression for filtering actors |  |

### Return type

[**crate::models::ActorsGetActorUsageResponse**](ActorsGetActorUsageResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

