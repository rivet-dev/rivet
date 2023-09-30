# \PortalNotificationsApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**portal_notifications_register_notifications**](PortalNotificationsApi.md#portal_notifications_register_notifications) | **POST** /portal/notifications/register | 
[**portal_notifications_unregister_notifications**](PortalNotificationsApi.md#portal_notifications_unregister_notifications) | **DELETE** /portal/notifications/register | 



## portal_notifications_register_notifications

> portal_notifications_register_notifications(portal_register_notifications_request)


Registers push notifications for the current identity.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**portal_register_notifications_request** | [**PortalRegisterNotificationsRequest**](PortalRegisterNotificationsRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## portal_notifications_unregister_notifications

> portal_notifications_unregister_notifications(service)


Unregister push notification for the current identity.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**service** | **String** | Represents a value for which notification service to unregister. | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

