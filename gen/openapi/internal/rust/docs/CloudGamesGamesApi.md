# \CloudGamesGamesApi

All URIs are relative to *https://api.rivet.gg*

Method | HTTP request | Description
------------- | ------------- | -------------
[**cloud_games_games_create_game**](CloudGamesGamesApi.md#cloud_games_games_create_game) | **POST** /cloud/games | 
[**cloud_games_games_game_banner_upload_complete**](CloudGamesGamesApi.md#cloud_games_games_game_banner_upload_complete) | **POST** /cloud/games/{game_id}/banner-upload/{upload_id}/complete | 
[**cloud_games_games_game_banner_upload_prepare**](CloudGamesGamesApi.md#cloud_games_games_game_banner_upload_prepare) | **POST** /cloud/games/{game_id}/banner-upload/prepare | 
[**cloud_games_games_game_logo_upload_complete**](CloudGamesGamesApi.md#cloud_games_games_game_logo_upload_complete) | **POST** /cloud/games/{game_id}/logo-upload/{upload_id}/complete | 
[**cloud_games_games_game_logo_upload_prepare**](CloudGamesGamesApi.md#cloud_games_games_game_logo_upload_prepare) | **POST** /cloud/games/{game_id}/logo-upload/prepare | 
[**cloud_games_games_get_game_by_id**](CloudGamesGamesApi.md#cloud_games_games_get_game_by_id) | **GET** /cloud/games/{game_id} | 
[**cloud_games_games_get_games**](CloudGamesGamesApi.md#cloud_games_games_get_games) | **GET** /cloud/games | 
[**cloud_games_games_validate_game**](CloudGamesGamesApi.md#cloud_games_games_validate_game) | **POST** /cloud/games/validate | 



## cloud_games_games_create_game

> crate::models::CloudGamesCreateGameResponse cloud_games_games_create_game(cloud_games_create_game_request)


Creates a new game.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**cloud_games_create_game_request** | [**CloudGamesCreateGameRequest**](CloudGamesCreateGameRequest.md) |  | [required] |

### Return type

[**crate::models::CloudGamesCreateGameResponse**](CloudGamesCreateGameResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_games_game_banner_upload_complete

> cloud_games_games_game_banner_upload_complete(game_id, upload_id)


Completes an game banner image upload. Must be called after the file upload process completes.

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


## cloud_games_games_game_banner_upload_prepare

> crate::models::CloudGamesGameBannerUploadPrepareResponse cloud_games_games_game_banner_upload_prepare(game_id, cloud_games_game_banner_upload_prepare_request)


Prepares a game banner image upload.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**cloud_games_game_banner_upload_prepare_request** | [**CloudGamesGameBannerUploadPrepareRequest**](CloudGamesGameBannerUploadPrepareRequest.md) |  | [required] |

### Return type

[**crate::models::CloudGamesGameBannerUploadPrepareResponse**](CloudGamesGameBannerUploadPrepareResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_games_game_logo_upload_complete

> cloud_games_games_game_logo_upload_complete(game_id, upload_id)


Completes a game logo image upload. Must be called after the file upload process completes.

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


## cloud_games_games_game_logo_upload_prepare

> crate::models::CloudGamesGameLogoUploadPrepareResponse cloud_games_games_game_logo_upload_prepare(game_id, cloud_games_game_logo_upload_prepare_request)


Prepares a game logo image upload.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**cloud_games_game_logo_upload_prepare_request** | [**CloudGamesGameLogoUploadPrepareRequest**](CloudGamesGameLogoUploadPrepareRequest.md) |  | [required] |

### Return type

[**crate::models::CloudGamesGameLogoUploadPrepareResponse**](CloudGamesGameLogoUploadPrepareResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_games_get_game_by_id

> crate::models::CloudGamesGetGameByIdResponse cloud_games_games_get_game_by_id(game_id, watch_index)


Returns a game by its game id.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**watch_index** | Option<**String**> | A query parameter denoting the requests watch index. |  |

### Return type

[**crate::models::CloudGamesGetGameByIdResponse**](CloudGamesGetGameByIdResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_games_get_games

> crate::models::CloudGamesGetGamesResponse cloud_games_games_get_games(watch_index)


Returns a list of games in which the current identity is a group member of its development team.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**watch_index** | Option<**String**> | A query parameter denoting the requests watch index. |  |

### Return type

[**crate::models::CloudGamesGetGamesResponse**](CloudGamesGetGamesResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_games_validate_game

> crate::models::CloudGamesValidateGameResponse cloud_games_games_validate_game(cloud_games_validate_game_request)


Validates information used to create a new game.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**cloud_games_validate_game_request** | [**CloudGamesValidateGameRequest**](CloudGamesValidateGameRequest.md) |  | [required] |

### Return type

[**crate::models::CloudGamesValidateGameResponse**](CloudGamesValidateGameResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

