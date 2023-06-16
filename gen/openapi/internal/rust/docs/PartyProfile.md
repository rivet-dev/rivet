# PartyProfile

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**activity** | [**crate::models::PartyActivity**](PartyActivity.md) |  | 
**create_ts** | **String** | RFC3339 timestamp. | 
**external** | [**crate::models::PartyExternalLinks**](PartyExternalLinks.md) |  | 
**invites** | [**Vec<crate::models::PartyInvite>**](PartyInvite.md) | A list of party invites. | 
**members** | [**Vec<crate::models::PartyMemberSummary>**](PartyMemberSummary.md) | A list of party members. | 
**party_id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**party_size** | Option<**i32**> | Unsigned 32 bit integer. | [optional]
**publicity** | [**crate::models::PartyPublicity**](PartyPublicity.md) |  | 
**thread_id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


