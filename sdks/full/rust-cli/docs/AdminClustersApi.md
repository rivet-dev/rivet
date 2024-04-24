# \AdminClustersApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**admin_clusters_create**](AdminClustersApi.md#admin_clusters_create) | **POST** /admin/clusters | 
[**admin_clusters_get_server_ips**](AdminClustersApi.md#admin_clusters_get_server_ips) | **GET** /admin/clusters/server_ips | 
[**admin_clusters_list**](AdminClustersApi.md#admin_clusters_list) | **GET** /admin/clusters | 



## admin_clusters_create

> crate::models::AdminClustersCreateResponse admin_clusters_create(admin_clusters_create_request)


Create a new cluster

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**admin_clusters_create_request** | [**AdminClustersCreateRequest**](AdminClustersCreateRequest.md) |  | [required] |

### Return type

[**crate::models::AdminClustersCreateResponse**](AdminClustersCreateResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## admin_clusters_get_server_ips

> crate::models::AdminClustersGetServerIpsResponse admin_clusters_get_server_ips(server_id, pool)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**server_id** | Option<**uuid::Uuid**> |  |  |
**pool** | Option<[**AdminPoolType**](.md)> |  |  |

### Return type

[**crate::models::AdminClustersGetServerIpsResponse**](AdminClustersGetServerIpsResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## admin_clusters_list

> crate::models::AdminClustersListResponse admin_clusters_list()


Get clusters

### Parameters

This endpoint does not need any parameter.

### Return type

[**crate::models::AdminClustersListResponse**](AdminClustersListResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

