# \CloudGamesAvatarsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**cloud_games_avatars_complete_custom_avatar_upload**](CloudGamesAvatarsApi.md#cloud_games_avatars_complete_custom_avatar_upload) | **POST** /games/{game_id}/avatar-upload/{upload_id}/complete | 
[**cloud_games_avatars_list_game_custom_avatars**](CloudGamesAvatarsApi.md#cloud_games_avatars_list_game_custom_avatars) | **GET** /games/{game_id}/avatars | 
[**cloud_games_avatars_prepare_custom_avatar_upload**](CloudGamesAvatarsApi.md#cloud_games_avatars_prepare_custom_avatar_upload) | **POST** /games/{game_id}/prepare | 



## cloud_games_avatars_complete_custom_avatar_upload

> cloud_games_avatars_complete_custom_avatar_upload(game_id, upload_id)


Completes a custom avatar image upload. Must be called after the file upload process completes.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**upload_id** | **uuid::Uuid** |  | [required] |

### Return type

 (empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_avatars_list_game_custom_avatars

> crate::models::CloudGamesListGameCustomAvatarsResponse cloud_games_avatars_list_game_custom_avatars(game_id)


Lists custom avatars for the given game.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |

### Return type

[**crate::models::CloudGamesListGameCustomAvatarsResponse**](CloudGamesListGameCustomAvatarsResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_avatars_prepare_custom_avatar_upload

> crate::models::CloudGamesPrepareCustomAvatarUploadResponse cloud_games_avatars_prepare_custom_avatar_upload(game_id, cloud_games_prepare_custom_avatar_upload_request)


Prepares a custom avatar image upload. Complete upload with `rivet.api.cloud#CompleteCustomAvatarUpload`.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**cloud_games_prepare_custom_avatar_upload_request** | [**CloudGamesPrepareCustomAvatarUploadRequest**](CloudGamesPrepareCustomAvatarUploadRequest.md) |  | [required] |

### Return type

[**crate::models::CloudGamesPrepareCustomAvatarUploadResponse**](CloudGamesPrepareCustomAvatarUploadResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

