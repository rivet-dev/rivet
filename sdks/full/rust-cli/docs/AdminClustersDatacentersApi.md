# \AdminClustersDatacentersApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**admin_clusters_datacenters_create**](AdminClustersDatacentersApi.md#admin_clusters_datacenters_create) | **POST** /admin/clusters/{cluster_id}/datacenters | 
[**admin_clusters_datacenters_list**](AdminClustersDatacentersApi.md#admin_clusters_datacenters_list) | **GET** /admin/clusters/{cluster_id}/datacenters | 
[**admin_clusters_datacenters_taint**](AdminClustersDatacentersApi.md#admin_clusters_datacenters_taint) | **GET** /admin/clusters/{cluster_id}/datacenters/{datacenter_id}/taint | 



## admin_clusters_datacenters_create

> crate::models::AdminClustersDatacentersCreateResponse admin_clusters_datacenters_create(cluster_id, admin_clusters_datacenters_create_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**cluster_id** | **uuid::Uuid** |  | [required] |
**admin_clusters_datacenters_create_request** | [**AdminClustersDatacentersCreateRequest**](AdminClustersDatacentersCreateRequest.md) |  | [required] |

### Return type

[**crate::models::AdminClustersDatacentersCreateResponse**](AdminClustersDatacentersCreateResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## admin_clusters_datacenters_list

> crate::models::AdminClustersDatacentersListResponse admin_clusters_datacenters_list(cluster_id)


Get datacenters of a cluster

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**cluster_id** | **uuid::Uuid** |  | [required] |

### Return type

[**crate::models::AdminClustersDatacentersListResponse**](AdminClustersDatacentersListResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## admin_clusters_datacenters_taint

> admin_clusters_datacenters_taint(cluster_id, datacenter_id)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**cluster_id** | **uuid::Uuid** |  | [required] |
**datacenter_id** | **String** |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

