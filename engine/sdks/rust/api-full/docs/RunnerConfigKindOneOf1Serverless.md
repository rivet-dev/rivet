# RunnerConfigKindOneOf1Serverless

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**actor_eviction_delay** | Option<**i32**> | Seconds. | [optional]
**actor_eviction_period** | Option<**i32**> | Seconds. | [optional]
**actor_eviction_rate** | Option<**f32**> | Actors per second. | [optional]
**drain_grace_period** | Option<**i32**> | Seconds. | [optional]
**drain_on_version_upgrade** | Option<**bool**> |  | [optional]
**headers** | Option<**std::collections::HashMap<String, String>**> |  | [optional]
**max_concurrent_actors** | Option<**i64**> |  | [optional]
**max_runners** | Option<**i32**> | Deprecated. | [optional]
**metadata_poll_interval** | Option<**i64**> | Milliseconds between metadata polling. If not set, uses the global default. | [optional]
**min_runners** | Option<**i32**> | Deprecated. | [optional]
**request_lifespan** | **i32** | Seconds. | 
**runners_margin** | Option<**i32**> | Deprecated. | [optional]
**slots_per_runner** | Option<**i32**> | Deprecated. | [optional]
**url** | **String** |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


