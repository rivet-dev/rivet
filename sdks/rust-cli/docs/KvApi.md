# \KvApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**kv_delete**](KvApi.md#kv_delete) | **DELETE** /kv/entries | 
[**kv_delete_batch**](KvApi.md#kv_delete_batch) | **DELETE** /kv/entries/batch | 
[**kv_get**](KvApi.md#kv_get) | **GET** /kv/entries | 
[**kv_get_batch**](KvApi.md#kv_get_batch) | **GET** /kv/entries/batch | 
[**kv_list**](KvApi.md#kv_list) | **GET** /kv/entries/list | 
[**kv_put**](KvApi.md#kv_put) | **PUT** /kv/entries | 
[**kv_put_batch**](KvApi.md#kv_put_batch) | **PUT** /kv/entries/batch | 



## kv_delete

> kv_delete(key, namespace_id)


Deletes a key-value entry by key.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**key** | **String** |  | [required] |
**namespace_id** | Option<**uuid::Uuid**> |  |  |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## kv_delete_batch

> kv_delete_batch(keys, namespace_id)


Deletes multiple key-value entries by key(s).

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**keys** | **String** |  | [required] |
**namespace_id** | Option<**uuid::Uuid**> |  |  |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## kv_get

> crate::models::KvGetResponse kv_get(key, watch_index, namespace_id)


Returns a specific key-value entry by key.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**key** | **String** |  | [required] |
**watch_index** | Option<**String**> | A query parameter denoting the requests watch index. |  |
**namespace_id** | Option<**uuid::Uuid**> |  |  |

### Return type

[**crate::models::KvGetResponse**](KvGetResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## kv_get_batch

> crate::models::KvGetBatchResponse kv_get_batch(keys, watch_index, namespace_id)


Gets multiple key-value entries by key(s).

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**keys** | **String** |  | [required] |
**watch_index** | Option<**String**> | A query parameter denoting the requests watch index. |  |
**namespace_id** | Option<**uuid::Uuid**> |  |  |

### Return type

[**crate::models::KvGetBatchResponse**](KvGetBatchResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## kv_list

> crate::models::KvListResponse kv_list(directory, namespace_id)


Lists all keys in a directory.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**directory** | **String** |  | [required] |
**namespace_id** | **uuid::Uuid** |  | [required] |

### Return type

[**crate::models::KvListResponse**](KvListResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## kv_put

> kv_put(kv_put_request)


Puts (sets or overwrites) a key-value entry by key.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**kv_put_request** | [**KvPutRequest**](KvPutRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## kv_put_batch

> kv_put_batch(kv_put_batch_request)


Puts (sets or overwrites) multiple key-value entries by key(s).

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**kv_put_batch_request** | [**KvPutBatchRequest**](KvPutBatchRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

