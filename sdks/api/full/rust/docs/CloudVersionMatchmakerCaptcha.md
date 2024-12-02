# CloudVersionMatchmakerCaptcha

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**requests_before_reverify** | **i32** | Denotes how many requests a connection can make before it is required to reverify a captcha. | 
**verification_ttl** | **i64** | Denotes how long a connection can continue to reconnect without having to reverify a captcha (in milliseconds). | 
**hcaptcha** | Option<[**crate::models::CloudVersionMatchmakerCaptchaHcaptcha**](CloudVersionMatchmakerCaptchaHcaptcha.md)> |  | [optional]
**turnstile** | Option<[**crate::models::CloudVersionMatchmakerCaptchaTurnstile**](CloudVersionMatchmakerCaptchaTurnstile.md)> |  | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


