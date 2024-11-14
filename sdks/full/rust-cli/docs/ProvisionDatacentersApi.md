# \ProvisionDatacentersApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**provision_datacenters_get_tls**](ProvisionDatacentersApi.md#provision_datacenters_get_tls) | **GET** /datacenters/{datacenter_id}/tls | 



## provision_datacenters_get_tls

> crate::models::ProvisionDatacentersGetTlsResponse provision_datacenters_get_tls(datacenter_id)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**datacenter_id** | **uuid::Uuid** |  | [required] |

### Return type

[**crate::models::ProvisionDatacentersGetTlsResponse**](ProvisionDatacentersGetTlsResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

