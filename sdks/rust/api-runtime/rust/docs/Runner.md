# Runner

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**addresses_http** | [**std::collections::HashMap<String, models::StringHttpAddressHashableMapValue>**](StringHttpAddressHashableMap_value.md) |  | 
**addresses_tcp** | [**std::collections::HashMap<String, models::StringHttpAddressHashableMapValue>**](StringHttpAddressHashableMap_value.md) |  | 
**addresses_udp** | [**std::collections::HashMap<String, models::StringHttpAddressHashableMapValue>**](StringHttpAddressHashableMap_value.md) |  | 
**create_ts** | **i64** |  | 
**datacenter** | **String** |  | 
**drain_ts** | Option<**i64**> |  | [optional]
**key** | **String** |  | 
**last_connected_ts** | Option<**i64**> |  | [optional]
**last_ping_ts** | **i64** |  | 
**last_rtt** | **i32** |  | 
**metadata** | Option<[**serde_json::Value**](serde_json::Value.md)> |  | [optional]
**name** | **String** |  | 
**namespace_id** | **String** |  | 
**remaining_slots** | **i32** |  | 
**runner_id** | **String** |  | 
**stop_ts** | Option<**i64**> |  | [optional]
**total_slots** | **i32** |  | 
**version** | **i32** |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


