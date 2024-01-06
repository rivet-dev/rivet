# GroupSummary

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**avatar_url** | Option<**String**> | The URL of this group's avatar image. | [optional]
**bio** | **String** | Follows regex ^(?:[^\\n\\r]+\\n?|\\n){1,5}$ | 
**display_name** | **String** |  | 
**external** | [**crate::models::GroupExternalLinks**](GroupExternalLinks.md) |  | 
**group_id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**is_current_identity_member** | **bool** | Whether or not the current identity is a member of this group. | 
**is_developer** | **bool** | Whether or not this group is a developer. | 
**member_count** | **i32** |  | 
**owner_identity_id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**publicity** | [**crate::models::GroupPublicity**](GroupPublicity.md) |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


