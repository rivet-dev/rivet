# GameStatConfig

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**aggregation** | [**crate::models::GameStatAggregationMethod**](GameStatAggregationMethod.md) |  | 
**display_name** | **String** |  | 
**format** | [**crate::models::GameStatFormatMethod**](GameStatFormatMethod.md) |  | 
**icon_id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**postfix_plural** | Option<**String**> | A string appended to the end of a game statistic's value that is not exactly 1. Example: 45 **dollars**. | [optional]
**postfix_singular** | Option<**String**> | A string appended to the end of a singular game statistic's value. Example: 1 **dollar**. | [optional]
**prefix_plural** | Option<**String**> | A string prepended to the beginning of a game statistic's value that is not exactly 1. Example: **values** 45. | [optional]
**prefix_singular** | Option<**String**> | A string appended to the beginning of a singular game statistic's value. Example: **value** 1. | [optional]
**priority** | **i32** |  | 
**record_id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**sorting** | [**crate::models::GameStatSortingMethod**](GameStatSortingMethod.md) |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


