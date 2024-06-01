# \AdminClustersServersApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**admin_clusters_servers_destroy**](AdminClustersServersApi.md#admin_clusters_servers_destroy) | **POST** /admin/clusters/{cluster_id}/servers/destroy | 
[**admin_clusters_servers_list**](AdminClustersServersApi.md#admin_clusters_servers_list) | **GET** /admin/clusters/{cluster_id}/servers | 
[**admin_clusters_servers_taint**](AdminClustersServersApi.md#admin_clusters_servers_taint) | **POST** /admin/clusters/{cluster_id}/servers/taint | 



## admin_clusters_servers_destroy

> admin_clusters_servers_destroy(cluster_id, server_id, datacenter, pool, public_ip)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**cluster_id** | **uuid::Uuid** |  | [required] |
**server_id** | Option<**String**> |  |  |
**datacenter** | Option<**String**> |  |  |
**pool** | Option<[**AdminClustersPoolType**](.md)> |  |  |
**public_ip** | Option<**String**> |  |  |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## admin_clusters_servers_list

> crate::models::AdminClustersListServersResponse admin_clusters_servers_list(cluster_id, server_id, datacenter, pool, public_ip)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**cluster_id** | **uuid::Uuid** |  | [required] |
**server_id** | Option<**String**> |  |  |
**datacenter** | Option<**String**> |  |  |
**pool** | Option<[**AdminClustersPoolType**](.md)> |  |  |
**public_ip** | Option<**String**> |  |  |

### Return type

[**crate::models::AdminClustersListServersResponse**](AdminClustersListServersResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## admin_clusters_servers_taint

> admin_clusters_servers_taint(cluster_id, server_id, datacenter, pool, public_ip)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**cluster_id** | **uuid::Uuid** |  | [required] |
**server_id** | Option<**String**> |  |  |
**datacenter** | Option<**String**> |  |  |
**pool** | Option<[**AdminClustersPoolType**](.md)> |  |  |
**public_ip** | Option<**String**> |  |  |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

