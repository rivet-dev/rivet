# \ServersDatacentersApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**servers_datacenters_list**](ServersDatacentersApi.md#servers_datacenters_list) | **GET** /games/{game_id}/environments/{environment_id}/datacenters | 



## servers_datacenters_list

> crate::models::ServersListDatacentersResponse servers_datacenters_list(game_id, environment_id)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**environment_id** | **uuid::Uuid** |  | [required] |

### Return type

[**crate::models::ServersListDatacentersResponse**](ServersListDatacentersResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

