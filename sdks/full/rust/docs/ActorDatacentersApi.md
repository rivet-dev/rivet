# \ActorDatacentersApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**actor_datacenters_list**](ActorDatacentersApi.md#actor_datacenters_list) | **GET** /datacenters | 



## actor_datacenters_list

> crate::models::ActorListDatacentersResponse actor_datacenters_list(game_id, environment_id)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | Option<**uuid::Uuid**> |  |  |
**environment_id** | Option<**uuid::Uuid**> |  |  |

### Return type

[**crate::models::ActorListDatacentersResponse**](ActorListDatacentersResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

