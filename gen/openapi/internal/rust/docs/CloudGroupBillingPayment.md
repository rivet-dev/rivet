# CloudGroupBillingPayment

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**amount** | **f64** | Payment amount (in hundredths USD, 100 = $1.00). | 
**created_ts** | **String** | RFC3339 timestamp. | 
**description** | Option<**String**> | A description of this payment. | [optional]
**from_invoice** | **bool** | Whether or not this payment is from an invoice. | 
**status** | [**crate::models::CloudGroupBillingStatus**](CloudGroupBillingStatus.md) |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


