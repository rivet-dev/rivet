# CloudRegionTier

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**bandwidth** | **i32** | Internet bandwidth (MB). | 
**cpu** | **i32** | CPU frequency (MHz). | 
**disk** | **i32** | Allocated disk space (MB). | 
**memory** | **i32** | Allocated memory (MB). | 
**price_per_second** | **i32** | **Deprecated** Price billed for every second this server is running (in quadrillionth USD, 1,000,000,000,000 = $1.00). | 
**rivet_cores_denominator** | **i32** | Together with the numerator, denotes the portion of the CPU a given server uses. | 
**rivet_cores_numerator** | **i32** | Together with the denominator, denotes the portion of the CPU a given server uses. | 
**tier_name_id** | **String** | A human readable short identifier used to references resources. Different than a `rivet.common#Uuid` because this is intended to be human readable. Different than `rivet.common#DisplayName` because this should not include special characters and be short. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


