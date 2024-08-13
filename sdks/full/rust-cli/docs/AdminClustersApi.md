# \AdminClustersApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**admin_clusters_create**](AdminClustersApi.md#admin_clusters_create) | **POST** /admin/clusters | 
[**admin_clusters_list**](AdminClustersApi.md#admin_clusters_list) | **GET** /admin/clusters | 



## admin_clusters_create

> crate::models::AdminClustersCreateClusterResponse admin_clusters_create(admin_clusters_create_cluster_request)


Create a new cluster

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**admin_clusters_create_cluster_request** | [**AdminClustersCreateClusterRequest**](AdminClustersCreateClusterRequest.md) |  | [required] |

### Return type

[**crate::models::AdminClustersCreateClusterResponse**](AdminClustersCreateClusterResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## admin_clusters_list

> crate::models::AdminClustersListClustersResponse admin_clusters_list()


Get clusters

### Parameters

This endpoint does not need any parameter.

### Return type

[**crate::models::AdminClustersListClustersResponse**](AdminClustersListClustersResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

