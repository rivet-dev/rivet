# \NamespacesApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**namespaces_create**](NamespacesApi.md#namespaces_create) | **POST** /namespaces | 
[**namespaces_get**](NamespacesApi.md#namespaces_get) | **GET** /namespaces/{namespace_id} | 
[**namespaces_list**](NamespacesApi.md#namespaces_list) | **GET** /namespaces | 



## namespaces_create

> models::NamespacesCreateResponse namespaces_create(namespaces_create_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**namespaces_create_request** | [**NamespacesCreateRequest**](NamespacesCreateRequest.md) |  | [required] |

### Return type

[**models::NamespacesCreateResponse**](NamespacesCreateResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## namespaces_get

> models::NamespacesGetResponse namespaces_get(namespace_id)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**namespace_id** | **String** |  | [required] |

### Return type

[**models::NamespacesGetResponse**](NamespacesGetResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## namespaces_list

> models::NamespacesListResponse namespaces_list(limit, cursor, name)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**limit** | Option<**i32**> |  |  |
**cursor** | Option<**String**> |  |  |
**name** | Option<**String**> |  |  |

### Return type

[**models::NamespacesListResponse**](NamespacesListResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

