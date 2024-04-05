# \AdminClusterApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**admin_cluster_get_server_ips**](AdminClusterApi.md#admin_cluster_get_server_ips) | **GET** /cluster/server_ips | 



## admin_cluster_get_server_ips

> crate::models::AdminClusterGetServerIpsResponse admin_cluster_get_server_ips(server_id, pool)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**server_id** | Option<**uuid::Uuid**> |  |  |
**pool** | Option<[**AdminPoolType**](.md)> |  |  |

### Return type

[**crate::models::AdminClusterGetServerIpsResponse**](AdminClusterGetServerIpsResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

