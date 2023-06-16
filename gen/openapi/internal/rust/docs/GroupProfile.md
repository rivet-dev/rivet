# GroupProfile

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**avatar_url** | Option<**String**> | The URL of this group's avatar image. | [optional]
**bio** | **String** | Detailed information about a profile. | 
**display_name** | **String** | Represent a resource's readable display name. | 
**external** | [**crate::models::GroupExternalLinks**](GroupExternalLinks.md) |  | 
**group_id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**is_current_identity_member** | Option<**bool**> | Whether or not the current identity is a member of this group. | [optional]
**is_current_identity_requesting_join** | Option<**bool**> | Whether or not the current identity is currently requesting to join this group. | [optional]
**is_developer** | Option<**bool**> | Whether or not this group is a developer. | [optional]
**join_requests** | [**Vec<crate::models::GroupJoinRequest>**](GroupJoinRequest.md) | A list of group join requests. | 
**member_count** | Option<**i32**> | Unsigned 32 bit integer. | [optional]
**members** | [**Vec<crate::models::GroupMember>**](GroupMember.md) | A list of group members. | 
**owner_identity_id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**publicity** | [**crate::models::GroupPublicity**](GroupPublicity.md) |  | 
**thread_id** | Option<[**uuid::Uuid**](uuid::Uuid.md)> |  | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


