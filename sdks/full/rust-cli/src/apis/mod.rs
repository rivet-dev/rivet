use std::error;
use std::fmt;

#[derive(Debug, Clone)]
pub struct ResponseContent<T> {
    pub status: reqwest::StatusCode,
    pub content: String,
    pub entity: Option<T>,
}

#[derive(Debug)]
pub enum Error<T> {
    Reqwest(reqwest::Error),
    Serde(serde_json::Error),
    Io(std::io::Error),
    ResponseError(ResponseContent<T>),
}

impl <T> fmt::Display for Error<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (module, e) = match self {
            Error::Reqwest(e) => ("reqwest", e.to_string()),
            Error::Serde(e) => ("serde", e.to_string()),
            Error::Io(e) => ("IO", e.to_string()),
            Error::ResponseError(e) => ("response", format!("status code {}", e.status)),
        };
        write!(f, "error in {}: {}", module, e)
    }
}

impl <T: fmt::Debug> error::Error for Error<T> {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(match self {
            Error::Reqwest(e) => e,
            Error::Serde(e) => e,
            Error::Io(e) => e,
            Error::ResponseError(_) => return None,
        })
    }
}

impl <T> From<reqwest::Error> for Error<T> {
    fn from(e: reqwest::Error) -> Self {
        Error::Reqwest(e)
    }
}

impl <T> From<serde_json::Error> for Error<T> {
    fn from(e: serde_json::Error) -> Self {
        Error::Serde(e)
    }
}

impl <T> From<std::io::Error> for Error<T> {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

pub fn urlencode<T: AsRef<str>>(s: T) -> String {
    ::url::form_urlencoded::byte_serialize(s.as_ref().as_bytes()).collect()
}

pub fn parse_deep_object(prefix: &str, value: &serde_json::Value) -> Vec<(String, String)> {
    if let serde_json::Value::Object(object) = value {
        let mut params = vec![];

        for (key, value) in object {
            match value {
                serde_json::Value::Object(_) => params.append(&mut parse_deep_object(
                    &format!("{}[{}]", prefix, key),
                    value,
                )),
                serde_json::Value::Array(array) => {
                    for (i, value) in array.iter().enumerate() {
                        params.append(&mut parse_deep_object(
                            &format!("{}[{}][{}]", prefix, key, i),
                            value,
                        ));
                    }
                },
                serde_json::Value::String(s) => params.push((format!("{}[{}]", prefix, key), s.clone())),
                _ => params.push((format!("{}[{}]", prefix, key), value.to_string())),
            }
        }

        return params;
    }

    unimplemented!("Only objects are supported with style=deepObject")
}

pub mod admin_api;
pub mod admin_clusters_api;
pub mod admin_clusters_datacenters_api;
pub mod admin_clusters_servers_api;
pub mod auth_identity_access_token_api;
pub mod auth_identity_email_api;
pub mod auth_tokens_api;
pub mod cloud_api;
pub mod cloud_auth_api;
pub mod cloud_devices_links_api;
pub mod cloud_games_api;
pub mod cloud_games_avatars_api;
pub mod cloud_games_builds_api;
pub mod cloud_games_cdn_api;
pub mod cloud_games_matchmaker_api;
pub mod cloud_games_namespaces_api;
pub mod cloud_games_namespaces_analytics_api;
pub mod cloud_games_namespaces_logs_api;
pub mod cloud_games_tokens_api;
pub mod cloud_games_versions_api;
pub mod cloud_groups_api;
pub mod cloud_logs_api;
pub mod cloud_tiers_api;
pub mod cloud_uploads_api;
pub mod games_servers_api;
pub mod games_servers_builds_api;
pub mod games_servers_logs_api;
pub mod group_api;
pub mod group_invites_api;
pub mod group_join_requests_api;
pub mod identity_api;
pub mod identity_activities_api;
pub mod identity_events_api;
pub mod identity_links_api;
pub mod job_run_api;
pub mod kv_api;
pub mod matchmaker_lobbies_api;
pub mod matchmaker_players_api;
pub mod matchmaker_regions_api;
pub mod portal_games_api;
pub mod provision_datacenters_api;
pub mod provision_servers_api;

pub mod configuration;
