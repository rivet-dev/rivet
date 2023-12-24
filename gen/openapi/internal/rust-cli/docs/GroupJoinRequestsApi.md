# \GroupJoinRequestsApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**group_join_requests_create_join_request**](GroupJoinRequestsApi.md#group_join_requests_create_join_request) | **POST** /group/groups/{group_id}/join-request | 
[**group_join_requests_resolve_join_request**](GroupJoinRequestsApi.md#group_join_requests_resolve_join_request) | **POST** /group/groups/{group_id}/join-request/{identity_id} | 



## group_join_requests_create_join_request

> group_join_requests_create_join_request(group_id)


Requests to join a group.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**group_id** | **uuid::Uuid** |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## group_join_requests_resolve_join_request

> group_join_requests_resolve_join_request(group_id, identity_id, group_resolve_join_request_request)


Resolves a join request for a given group.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**group_id** | **uuid::Uuid** |  | [required] |
**identity_id** | **uuid::Uuid** |  | [required] |
**group_resolve_join_request_request** | [**GroupResolveJoinRequestRequest**](GroupResolveJoinRequestRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

