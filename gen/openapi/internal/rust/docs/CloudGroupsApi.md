# \CloudGroupsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**cloud_groups_convert_group**](CloudGroupsApi.md#cloud_groups_convert_group) | **POST** /groups/{group_id}/convert | 
[**cloud_groups_validate**](CloudGroupsApi.md#cloud_groups_validate) | **POST** /groups/validate | 



## cloud_groups_convert_group

> cloud_groups_convert_group(group_id)


Converts the given group into a developer group.

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


## cloud_groups_validate

> crate::models::CloudValidateGroupResponse cloud_groups_validate(cloud_validate_group_request)


Validates information used to create a new group.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**cloud_validate_group_request** | [**CloudValidateGroupRequest**](CloudValidateGroupRequest.md) |  | [required] |

### Return type

[**crate::models::CloudValidateGroupResponse**](CloudValidateGroupResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

