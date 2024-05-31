# \AdminClustersDatacentersApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**admin_clusters_datacenters_create**](AdminClustersDatacentersApi.md#admin_clusters_datacenters_create) | **POST** /admin/clusters/{cluster_id}/datacenters | 
[**admin_clusters_datacenters_list**](AdminClustersDatacentersApi.md#admin_clusters_datacenters_list) | **GET** /admin/clusters/{cluster_id}/datacenters | 
[**admin_clusters_datacenters_update**](AdminClustersDatacentersApi.md#admin_clusters_datacenters_update) | **PATCH** /admin/clusters/{cluster_id}/datacenters/{datacenter_id} | 



## admin_clusters_datacenters_create

> crate::models::AdminClustersCreateDatacenterResponse admin_clusters_datacenters_create(cluster_id, admin_clusters_create_datacenter_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**cluster_id** | **uuid::Uuid** |  | [required] |
**admin_clusters_create_datacenter_request** | [**AdminClustersCreateDatacenterRequest**](AdminClustersCreateDatacenterRequest.md) |  | [required] |

### Return type

[**crate::models::AdminClustersCreateDatacenterResponse**](AdminClustersCreateDatacenterResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## admin_clusters_datacenters_list

> crate::models::AdminClustersListDatacentersResponse admin_clusters_datacenters_list(cluster_id)


Get datacenters of a cluster

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**cluster_id** | **uuid::Uuid** |  | [required] |

### Return type

[**crate::models::AdminClustersListDatacentersResponse**](AdminClustersListDatacentersResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## admin_clusters_datacenters_update

> admin_clusters_datacenters_update(cluster_id, datacenter_id, admin_clusters_update_datacenter_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**cluster_id** | **uuid::Uuid** |  | [required] |
**datacenter_id** | **uuid::Uuid** |  | [required] |
**admin_clusters_update_datacenter_request** | [**AdminClustersUpdateDatacenterRequest**](AdminClustersUpdateDatacenterRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

