# \ChatApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**chat_get_thread_history**](ChatApi.md#chat_get_thread_history) | **GET** /threads/{thread_id}/history | 
[**chat_get_thread_topic**](ChatApi.md#chat_get_thread_topic) | **GET** /threads/{thread_id}/topic | 
[**chat_send_message**](ChatApi.md#chat_send_message) | **POST** /messages | 
[**chat_set_thread_read**](ChatApi.md#chat_set_thread_read) | **POST** /threads/{thread_id}/read | 
[**chat_set_typing_status**](ChatApi.md#chat_set_typing_status) | **PUT** /threads/{thread_id}/typing-status | 
[**chat_watch_thread**](ChatApi.md#chat_watch_thread) | **GET** /threads/{thread_id}/live | 



## chat_get_thread_history

> crate::models::ChatGetThreadHistoryResponse chat_get_thread_history(thread_id, count, ts, query_direction)


Returns message history for a given thread in a certain direction. Defaults to querying messages before ts.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**thread_id** | **uuid::Uuid** |  | [required] |
**count** | **f64** | How many messages to collect in each direction. If querying `rivet.api.chat.common#QueryDirection$before_and_after`, `rivet.api.chat.common#QueryDirection$chat_messages` will be `count * 2`. | [required] |
**ts** | Option<**String**> | RFC3339 timestamp. |  |
**query_direction** | Option<**String**> | Represents which direction to query messages from relative to the given timestamp. |  |

### Return type

[**crate::models::ChatGetThreadHistoryResponse**](ChatGetThreadHistoryResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## chat_get_thread_topic

> crate::models::ChatGetThreadTopicResponse chat_get_thread_topic(thread_id)


Fetches the topic of a thread.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**thread_id** | **uuid::Uuid** |  | [required] |

### Return type

[**crate::models::ChatGetThreadTopicResponse**](ChatGetThreadTopicResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## chat_send_message

> crate::models::ChatSendMessageResponse chat_send_message(chat_send_message_request)


Sends a chat message to a given topic.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**chat_send_message_request** | [**ChatSendMessageRequest**](ChatSendMessageRequest.md) |  | [required] |

### Return type

[**crate::models::ChatSendMessageResponse**](ChatSendMessageResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## chat_set_thread_read

> chat_set_thread_read(thread_id, chat_set_thread_read_request)


Updates the current identity's last read timestamp in the given thread.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**thread_id** | **uuid::Uuid** |  | [required] |
**chat_set_thread_read_request** | [**ChatSetThreadReadRequest**](ChatSetThreadReadRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## chat_set_typing_status

> chat_set_typing_status(thread_id, chat_set_typing_status_request)


Updates the current identity's typing status in the given thread.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**thread_id** | **uuid::Uuid** |  | [required] |
**chat_set_typing_status_request** | [**ChatSetTypingStatusRequest**](ChatSetTypingStatusRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## chat_watch_thread

> crate::models::ChatWatchThreadResponse chat_watch_thread(thread_id, watch_index)


Fetches all relevant changes from a thread that have happened since the given watch index.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**thread_id** | **uuid::Uuid** |  | [required] |
**watch_index** | Option<**String**> | A query parameter denoting the requests watch index. |  |

### Return type

[**crate::models::ChatWatchThreadResponse**](ChatWatchThreadResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

