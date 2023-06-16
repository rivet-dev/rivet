# \CloudGamesCdnApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**cloud_games_cdn_create_game_cdn_site**](CloudGamesCdnApi.md#cloud_games_cdn_create_game_cdn_site) | **POST** /games/{game_id}/cdn/sites | 
[**cloud_games_cdn_list_game_cdn_sites**](CloudGamesCdnApi.md#cloud_games_cdn_list_game_cdn_sites) | **GET** /games/{game_id}/cdn/sites | 



## cloud_games_cdn_create_game_cdn_site

> crate::models::CloudGamesCreateGameCdnSiteResponse cloud_games_cdn_create_game_cdn_site(game_id, cloud_games_create_game_cdn_site_request)


Creates a new CDN site for the given game.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |
**cloud_games_create_game_cdn_site_request** | [**CloudGamesCreateGameCdnSiteRequest**](CloudGamesCreateGameCdnSiteRequest.md) |  | [required] |

### Return type

[**crate::models::CloudGamesCreateGameCdnSiteResponse**](CloudGamesCreateGameCdnSiteResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## cloud_games_cdn_list_game_cdn_sites

> crate::models::CloudGamesListGameCdnSitesResponse cloud_games_cdn_list_game_cdn_sites(game_id)


Lists CDN sites for a game.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_id** | **uuid::Uuid** |  | [required] |

### Return type

[**crate::models::CloudGamesListGameCdnSitesResponse**](CloudGamesListGameCdnSitesResponse.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

