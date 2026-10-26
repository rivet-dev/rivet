// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! Generic utilities for network based [`ObjectStore`] implementations
//!
//! [`ObjectStore`]: crate::ObjectStore

pub(crate) mod backoff;

#[cfg(not(target_arch = "wasm32"))]
mod dns;
#[cfg(not(target_arch = "wasm32"))]
pub use dns::{DnsError, DnsFuture, DnsResolver};

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
pub(crate) mod mock_server;

pub(crate) mod retry;

#[cfg(any(feature = "aws-base", feature = "gcp-base", feature = "azure-base"))]
pub(crate) mod pagination;

pub(crate) mod get;

#[cfg(any(feature = "aws-base", feature = "gcp-base", feature = "azure-base"))]
pub(crate) mod list;

#[cfg(any(feature = "aws-base", feature = "gcp-base", feature = "azure-base"))]
pub(crate) mod token;

pub(crate) mod header;

#[cfg(any(feature = "aws-base", feature = "gcp-base"))]
pub(crate) mod s3;

pub(crate) mod builder;
mod http;

#[cfg(any(feature = "aws-base", feature = "gcp-base", feature = "azure-base"))]
pub(crate) mod parts;
pub use http::*;

#[cfg(any(feature = "aws-base", feature = "gcp-base", feature = "azure-base"))]
mod crypto;

#[cfg(any(feature = "aws-base", feature = "gcp-base", feature = "azure-base"))]
pub use crypto::*;

use ::http::header::{HeaderMap, HeaderValue};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

#[cfg(all(feature = "reqwest", not(target_arch = "wasm32")))]
use reqwest::{NoProxy, Proxy};

use crate::config::{ConfigValue, fmt_duration};
use crate::path::Path;
use crate::{GetOptions, Result};

#[cfg(feature = "reqwest")]
fn map_client_error(e: reqwest::Error) -> super::Error {
    super::Error::Generic {
        store: "HTTP client",
        source: Box::new(e),
    }
}

#[cfg(feature = "reqwest")]
static DEFAULT_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

/// Configuration keys for [`ClientOptions`]
#[derive(PartialEq, Eq, Hash, Clone, Debug, Copy, Deserialize, Serialize)]
#[non_exhaustive]
pub enum ClientConfigKey {
    /// Allow non-TLS, i.e. non-HTTPS connections
    ///
    /// Supported keys:
    /// - `allow_http`
    AllowHttp,
    /// Skip certificate validation on https connections.
    ///
    /// <div class="warning">
    ///
    /// **Warning**
    ///
    /// You should think very carefully before using this method. If
    /// invalid certificates are trusted, *any* certificate for *any* site
    /// will be trusted for use. This includes expired certificates. This
    /// introduces significant vulnerabilities, and should only be used
    /// as a last resort or for testing
    ///
    /// </div>
    ///
    /// Supported keys:
    /// - `allow_invalid_certificates`
    AllowInvalidCertificates,
    /// Disable certificate validation using the operating system's certificate facilities.
    ///
    /// See [`ClientOptions::with_no_system_certificates`]
    ///
    /// Supported keys:
    ///
    /// - `disable_system_certificates`
    NoSystemCertificates,
    /// Timeout for only the connect phase of a Client
    ///
    /// Supported keys:
    /// - `connect_timeout`
    ConnectTimeout,
    /// default [`Content-Type`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Content-Type) for uploads
    ///
    /// Supported keys:
    /// - `default_content_type`
    DefaultContentType,
    /// Only use HTTP/1 connections
    ///
    /// Supported keys:
    /// - `http1_only`
    Http1Only,
    /// Interval for HTTP/2 Ping frames should be sent to keep a connection alive.
    ///
    /// Supported keys:
    /// - `http2_keep_alive_interval`
    Http2KeepAliveInterval,
    /// Timeout for receiving an acknowledgement of the keep-alive ping.
    ///
    /// Supported keys:
    /// - `http2_keep_alive_timeout`
    Http2KeepAliveTimeout,
    /// Enable HTTP/2 keep alive pings for idle connections
    ///
    /// Supported keys:
    /// - `http2_keep_alive_while_idle`
    Http2KeepAliveWhileIdle,
    /// Sets the maximum frame size to use for HTTP/2.
    ///
    /// Supported keys:
    /// - `http2_max_frame_size`
    Http2MaxFrameSize,
    /// Only use HTTP/2 connections
    ///
    /// Supported keys:
    /// - `http2_only`
    Http2Only,
    /// The pool max idle timeout
    ///
    /// This is the length of time an idle connection will be kept alive
    ///
    /// Supported keys:
    /// - `pool_idle_timeout`
    PoolIdleTimeout,
    /// maximum number of idle connections per host
    ///
    /// Supported keys:
    /// - `pool_max_idle_per_host`
    PoolMaxIdlePerHost,
    /// HTTP proxy to use for requests
    ///
    /// Supported keys:
    /// - `proxy_url`
    ProxyUrl,
    /// PEM-formatted CA certificate for proxy connections
    ///
    /// Supported keys:
    /// - `proxy_ca_certificate`
    ProxyCaCertificate,
    /// List of hosts that bypass proxy
    ///
    /// Supported keys:
    /// - `proxy_excludes`
    ProxyExcludes,
    /// Randomize order addresses that the DNS resolution yields.
    ///
    /// This will spread the connections across more servers.
    ///
    /// <div class="warning">
    ///
    /// **Warning**
    ///
    /// This will override the DNS resolver configured by [`reqwest`].
    ///
    /// </div>
    ///
    /// Supported keys:
    /// - `randomize_addresses`
    RandomizeAddresses,
    /// Read timeout
    ///
    /// The timeout applies to each read operation, and resets after a
    /// successful read. This is useful for detecting stalled connections
    /// when the size of the response is not known beforehand.
    ///
    /// Supported keys:
    /// - `read_timeout`
    ReadTimeout,
    /// Request timeout
    ///
    /// The timeout is applied from when the request starts connecting until the
    /// response body has finished
    ///
    /// Supported keys:
    /// - `timeout`
    Timeout,
    /// User-Agent header to be used by this client
    ///
    /// Supported keys:
    /// - `user_agent`
    UserAgent,
}

impl AsRef<str> for ClientConfigKey {
    fn as_ref(&self) -> &str {
        match self {
            Self::AllowHttp => "allow_http",
            Self::AllowInvalidCertificates => "allow_invalid_certificates",
            Self::NoSystemCertificates => "disable_system_certificates",
            Self::ConnectTimeout => "connect_timeout",
            Self::DefaultContentType => "default_content_type",
            Self::Http1Only => "http1_only",
            Self::Http2Only => "http2_only",
            Self::Http2KeepAliveInterval => "http2_keep_alive_interval",
            Self::Http2KeepAliveTimeout => "http2_keep_alive_timeout",
            Self::Http2KeepAliveWhileIdle => "http2_keep_alive_while_idle",
            Self::Http2MaxFrameSize => "http2_max_frame_size",
            Self::PoolIdleTimeout => "pool_idle_timeout",
            Self::PoolMaxIdlePerHost => "pool_max_idle_per_host",
            Self::ProxyUrl => "proxy_url",
            Self::ProxyCaCertificate => "proxy_ca_certificate",
            Self::ProxyExcludes => "proxy_excludes",
            Self::RandomizeAddresses => "randomize_addresses",
            Self::ReadTimeout => "read_timeout",
            Self::Timeout => "timeout",
            Self::UserAgent => "user_agent",
        }
    }
}

impl FromStr for ClientConfigKey {
    type Err = super::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "allow_http" => Ok(Self::AllowHttp),
            "allow_invalid_certificates" => Ok(Self::AllowInvalidCertificates),
            "disable_system_certificates" => Ok(Self::NoSystemCertificates),
            "connect_timeout" => Ok(Self::ConnectTimeout),
            "default_content_type" => Ok(Self::DefaultContentType),
            "http1_only" => Ok(Self::Http1Only),
            "http2_only" => Ok(Self::Http2Only),
            "http2_keep_alive_interval" => Ok(Self::Http2KeepAliveInterval),
            "http2_keep_alive_timeout" => Ok(Self::Http2KeepAliveTimeout),
            "http2_keep_alive_while_idle" => Ok(Self::Http2KeepAliveWhileIdle),
            "http2_max_frame_size" => Ok(Self::Http2MaxFrameSize),
            "pool_idle_timeout" => Ok(Self::PoolIdleTimeout),
            "pool_max_idle_per_host" => Ok(Self::PoolMaxIdlePerHost),
            "proxy_url" => Ok(Self::ProxyUrl),
            "proxy_ca_certificate" => Ok(Self::ProxyCaCertificate),
            "proxy_excludes" => Ok(Self::ProxyExcludes),
            "randomize_addresses" => Ok(Self::RandomizeAddresses),
            "read_timeout" => Ok(Self::ReadTimeout),
            "timeout" => Ok(Self::Timeout),
            "user_agent" => Ok(Self::UserAgent),
            _ => Err(super::Error::UnknownConfigurationKey {
                store: "HTTP",
                key: s.into(),
            }),
        }
    }
}

/// Represents a CA certificate provided by the user.
///
/// This is used to configure the client to trust a specific certificate. See
/// [Self::from_pem] for an example
#[derive(Debug, Clone)]
#[cfg(all(feature = "reqwest", not(target_arch = "wasm32")))]
pub struct Certificate(reqwest::tls::Certificate);

#[cfg(all(feature = "reqwest", not(target_arch = "wasm32")))]
impl Certificate {
    /// Create a `Certificate` from a PEM encoded certificate.
    ///
    /// # Example from a PEM file
    ///
    /// ```no_run
    /// # use object_store::Certificate;
    /// # use std::fs::File;
    /// # use std::io::Read;
    /// let mut buf = Vec::new();
    /// File::open("my_cert.pem").unwrap()
    ///   .read_to_end(&mut buf).unwrap();
    /// let cert = Certificate::from_pem(&buf).unwrap();
    ///
    /// ```
    pub fn from_pem(pem: &[u8]) -> Result<Self> {
        Ok(Self(
            reqwest::tls::Certificate::from_pem(pem).map_err(map_client_error)?,
        ))
    }

    /// Create a collection of `Certificate` from a PEM encoded certificate
    /// bundle.
    ///
    /// Files that contain such collections have extensions such as `.crt`,
    /// `.cer` and `.pem` files.
    pub fn from_pem_bundle(pem_bundle: &[u8]) -> Result<Vec<Self>> {
        Ok(reqwest::tls::Certificate::from_pem_bundle(pem_bundle)
            .map_err(map_client_error)?
            .into_iter()
            .map(Self)
            .collect())
    }

    /// Create a `Certificate` from a binary DER encoded certificate.
    pub fn from_der(der: &[u8]) -> Result<Self> {
        Ok(Self(
            reqwest::tls::Certificate::from_der(der).map_err(map_client_error)?,
        ))
    }
}

/// HTTP client configuration for remote object stores
#[derive(Debug, Clone)]
pub struct ClientOptions {
    user_agent: Option<ConfigValue<HeaderValue>>,
    #[cfg(all(feature = "reqwest", not(target_arch = "wasm32")))]
    root_certificates: Vec<Certificate>,
    no_system_certificates: ConfigValue<bool>,
    content_type_map: HashMap<String, String>,
    default_content_type: Option<String>,
    default_headers: Option<HeaderMap>,
    proxy_url: Option<String>,
    proxy_ca_certificate: Option<String>,
    proxy_excludes: Option<String>,
    allow_http: ConfigValue<bool>,
    allow_invalid_certificates: ConfigValue<bool>,
    timeout: Option<ConfigValue<Duration>>,
    connect_timeout: Option<ConfigValue<Duration>>,
    read_timeout: Option<ConfigValue<Duration>>,
    pool_idle_timeout: Option<ConfigValue<Duration>>,
    pool_max_idle_per_host: Option<ConfigValue<usize>>,
    http2_keep_alive_interval: Option<ConfigValue<Duration>>,
    http2_keep_alive_timeout: Option<ConfigValue<Duration>>,
    http2_keep_alive_while_idle: ConfigValue<bool>,
    http2_max_frame_size: Option<ConfigValue<u32>>,
    http1_only: ConfigValue<bool>,
    http2_only: ConfigValue<bool>,
    randomize_addresses: ConfigValue<bool>,
    #[cfg(not(target_arch = "wasm32"))]
    dns_resolver: Option<Arc<dyn DnsResolver>>,
}

impl Default for ClientOptions {
    fn default() -> Self {
        // Defaults based on
        // <https://docs.aws.amazon.com/sdkref/latest/guide/feature-smart-config-defaults.html>
        // <https://docs.aws.amazon.com/whitepapers/latest/s3-optimizing-performance-best-practices/timeouts-and-retries-for-latency-sensitive-applications.html>
        // Which recommend a connection timeout of 3.1s and a request timeout of 2s
        //
        // As object store requests may involve the transfer of non-trivial volumes of data
        // we opt for a slightly higher default timeout of 30 seconds
        Self {
            user_agent: None,
            #[cfg(all(feature = "reqwest", not(target_arch = "wasm32")))]
            root_certificates: Default::default(),
            no_system_certificates: false.into(),
            content_type_map: Default::default(),
            default_content_type: None,
            default_headers: None,
            proxy_url: None,
            proxy_ca_certificate: None,
            proxy_excludes: None,
            allow_http: Default::default(),
            allow_invalid_certificates: Default::default(),
            timeout: Some(Duration::from_secs(30).into()),
            connect_timeout: Some(Duration::from_secs(5).into()),
            read_timeout: None,
            pool_idle_timeout: None,
            pool_max_idle_per_host: None,
            http2_keep_alive_interval: None,
            http2_keep_alive_timeout: None,
            http2_keep_alive_while_idle: Default::default(),
            http2_max_frame_size: None,
            // HTTP/2 is known to be significantly slower than HTTP/1, so we default
            // to HTTP/1 for now.
            // https://github.com/apache/arrow-rs/issues/5194
            http1_only: true.into(),
            http2_only: Default::default(),
            randomize_addresses: true.into(),
            #[cfg(not(target_arch = "wasm32"))]
            dns_resolver: Default::default(),
        }
    }
}

impl ClientOptions {
    /// Create a new [`ClientOptions`] with default values
    pub fn new() -> Self {
        Default::default()
    }

    /// Set an option by key
    pub fn with_config(mut self, key: ClientConfigKey, value: impl Into<String>) -> Self {
        match key {
            ClientConfigKey::AllowHttp => self.allow_http.parse(value),
            ClientConfigKey::AllowInvalidCertificates => {
                self.allow_invalid_certificates.parse(value)
            }
            ClientConfigKey::NoSystemCertificates => self.no_system_certificates.parse(value),
            ClientConfigKey::ConnectTimeout => {
                self.connect_timeout = Some(ConfigValue::Deferred(value.into()))
            }
            ClientConfigKey::ReadTimeout => {
                self.read_timeout = Some(ConfigValue::Deferred(value.into()))
            }
            ClientConfigKey::DefaultContentType => self.default_content_type = Some(value.into()),
            ClientConfigKey::Http1Only => self.http1_only.parse(value),
            ClientConfigKey::Http2Only => self.http2_only.parse(value),
            ClientConfigKey::Http2KeepAliveInterval => {
                self.http2_keep_alive_interval = Some(ConfigValue::Deferred(value.into()))
            }
            ClientConfigKey::Http2KeepAliveTimeout => {
                self.http2_keep_alive_timeout = Some(ConfigValue::Deferred(value.into()))
            }
            ClientConfigKey::Http2KeepAliveWhileIdle => {
                self.http2_keep_alive_while_idle.parse(value)
            }
            ClientConfigKey::Http2MaxFrameSize => {
                self.http2_max_frame_size = Some(ConfigValue::Deferred(value.into()))
            }
            ClientConfigKey::PoolIdleTimeout => {
                self.pool_idle_timeout = Some(ConfigValue::Deferred(value.into()))
            }
            ClientConfigKey::PoolMaxIdlePerHost => {
                self.pool_max_idle_per_host = Some(ConfigValue::Deferred(value.into()))
            }
            ClientConfigKey::ProxyUrl => self.proxy_url = Some(value.into()),
            ClientConfigKey::ProxyCaCertificate => self.proxy_ca_certificate = Some(value.into()),
            ClientConfigKey::ProxyExcludes => self.proxy_excludes = Some(value.into()),
            ClientConfigKey::RandomizeAddresses => {
                self.randomize_addresses.parse(value);
            }
            ClientConfigKey::Timeout => self.timeout = Some(ConfigValue::Deferred(value.into())),
            ClientConfigKey::UserAgent => {
                self.user_agent = Some(ConfigValue::Deferred(value.into()))
            }
        }
        self
    }

    /// Get an option by key
    pub fn get_config_value(&self, key: &ClientConfigKey) -> Option<String> {
        match key {
            ClientConfigKey::AllowHttp => Some(self.allow_http.to_string()),
            ClientConfigKey::AllowInvalidCertificates => {
                Some(self.allow_invalid_certificates.to_string())
            }
            ClientConfigKey::NoSystemCertificates => Some(self.no_system_certificates.to_string()),
            ClientConfigKey::ConnectTimeout => self.connect_timeout.as_ref().map(fmt_duration),
            ClientConfigKey::ReadTimeout => self.read_timeout.as_ref().map(fmt_duration),
            ClientConfigKey::DefaultContentType => self.default_content_type.clone(),
            ClientConfigKey::Http1Only => Some(self.http1_only.to_string()),
            ClientConfigKey::Http2KeepAliveInterval => {
                self.http2_keep_alive_interval.as_ref().map(fmt_duration)
            }
            ClientConfigKey::Http2KeepAliveTimeout => {
                self.http2_keep_alive_timeout.as_ref().map(fmt_duration)
            }
            ClientConfigKey::Http2KeepAliveWhileIdle => {
                Some(self.http2_keep_alive_while_idle.to_string())
            }
            ClientConfigKey::Http2MaxFrameSize => {
                self.http2_max_frame_size.as_ref().map(|v| v.to_string())
            }
            ClientConfigKey::Http2Only => Some(self.http2_only.to_string()),
            ClientConfigKey::PoolIdleTimeout => self.pool_idle_timeout.as_ref().map(fmt_duration),
            ClientConfigKey::PoolMaxIdlePerHost => {
                self.pool_max_idle_per_host.as_ref().map(|v| v.to_string())
            }
            ClientConfigKey::ProxyUrl => self.proxy_url.clone(),
            ClientConfigKey::ProxyCaCertificate => self.proxy_ca_certificate.clone(),
            ClientConfigKey::ProxyExcludes => self.proxy_excludes.clone(),
            ClientConfigKey::RandomizeAddresses => Some(self.randomize_addresses.to_string()),
            ClientConfigKey::Timeout => self.timeout.as_ref().map(fmt_duration),
            ClientConfigKey::UserAgent => self
                .user_agent
                .as_ref()
                .and_then(|v| v.get().ok())
                .and_then(|v| v.to_str().ok().map(|s| s.to_string())),
        }
    }

    /// Sets the [`User-Agent`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/User-Agent) header to be used by this client
    ///
    /// Default is based on the version of this crate
    pub fn with_user_agent(mut self, agent: HeaderValue) -> Self {
        self.user_agent = Some(agent.into());
        self
    }

    /// Add a custom root certificate.
    ///
    /// This can be used to connect to a server that has a self-signed
    /// certificate for example.
    #[cfg(all(feature = "reqwest", not(target_arch = "wasm32")))]
    pub fn with_root_certificate(mut self, certificate: Certificate) -> Self {
        self.root_certificates.push(certificate);
        self
    }

    /// Set the default [`Content-Type`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Content-Type) for uploads
    pub fn with_default_content_type(mut self, mime: impl Into<String>) -> Self {
        self.default_content_type = Some(mime.into());
        self
    }

    /// Set the [`Content-Type`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Content-Type) for a given file extension
    pub fn with_content_type_for_suffix(
        mut self,
        extension: impl Into<String>,
        mime: impl Into<String>,
    ) -> Self {
        self.content_type_map.insert(extension.into(), mime.into());
        self
    }

    /// Sets the default headers for every request
    pub fn with_default_headers(mut self, headers: HeaderMap) -> Self {
        self.default_headers = Some(headers);
        self
    }

    /// Sets what protocol is allowed.
    ///
    /// If `allow_http` is :
    /// * `false` (default):  Only HTTPS is allowed
    /// * `true`:  HTTP and HTTPS are allowed
    pub fn with_allow_http(mut self, allow_http: bool) -> Self {
        self.allow_http = allow_http.into();
        self
    }

    /// Allows connections to invalid SSL certificates
    ///
    /// If `allow_invalid_certificates` is :
    /// * `false` (default):  Only valid HTTPS certificates are allowed
    /// * `true`:  All HTTPS certificates are allowed
    ///
    /// <div class="warning">
    ///
    /// **Warning**
    ///
    /// You should think very carefully before using this method. If
    /// invalid certificates are trusted, *any* certificate for *any* site
    /// will be trusted for use. This includes expired certificates. This
    /// introduces significant vulnerabilities, and should only be used
    /// as a last resort or for testing
    ///
    /// </div>
    pub fn with_allow_invalid_certificates(mut self, allow_invalid_certificates: bool) -> Self {
        self.allow_invalid_certificates = allow_invalid_certificates.into();
        self
    }

    /// Disable certificates provided by the system
    ///
    /// By default TLS certificates are validated using [`rustls-platform-verifier`],
    /// which makes use of the system's trust store, in addition to any certificates
    /// registered using [`Self::with_root_certificate`]. If disabled, instead [`rustls-webpki`]
    /// is used with only the certificates registered using [`Self::with_root_certificate`].
    ///
    /// [`rustls-platform-verifier`]: https://crates.io/crates/rustls-platform-verifier
    /// [`rustls-webpki`]: https://crates.io/crates/rustls-webpki
    pub fn with_no_system_certificates(mut self, no_certs: bool) -> Self {
        self.no_system_certificates = no_certs.into();
        self
    }

    /// Only use HTTP/1 connections (default)
    ///
    /// # See Also
    /// * [`Self::with_http2_only`] if you only want to use HTTP/2
    /// * [`Self::with_allow_http2`] if you want to use HTTP/1 or HTTP/2
    ///
    /// <div class="warning">
    /// HTTP/2 is not used by default. See details [#104](https://github.com/apache/arrow-rs-object-store/issues/104)
    /// </div>
    pub fn with_http1_only(mut self) -> Self {
        self.http2_only = false.into();
        self.http1_only = true.into();
        self
    }

    /// Only use HTTP/2 connections
    ///
    /// # See Also
    /// * [`Self::with_http1_only`] if you only want to use HTTP/1
    /// * [`Self::with_allow_http2`] if you want to use HTTP/1 or HTTP/2
    ///
    /// <div class="warning">
    /// HTTP/2 is not used by default. See details [#104](https://github.com/apache/arrow-rs-object-store/issues/104)
    /// </div>
    pub fn with_http2_only(mut self) -> Self {
        self.http1_only = false.into();
        self.http2_only = true.into();
        self
    }

    /// Use HTTP/2 if supported, otherwise use HTTP/1.
    ///
    /// # See Also
    /// * [`Self::with_http1_only`] if you only want to use HTTP/1
    /// * [`Self::with_http2_only`] if you only want to use HTTP/2
    ///
    /// <div class="warning">
    /// HTTP/2 is not used by default. See details [#104](https://github.com/apache/arrow-rs-object-store/issues/104)
    /// </div>
    pub fn with_allow_http2(mut self) -> Self {
        self.http1_only = false.into();
        self.http2_only = false.into();
        self
    }

    /// Set a proxy URL to use for requests
    pub fn with_proxy_url(mut self, proxy_url: impl Into<String>) -> Self {
        self.proxy_url = Some(proxy_url.into());
        self
    }

    /// Set a trusted proxy CA certificate
    pub fn with_proxy_ca_certificate(mut self, proxy_ca_certificate: impl Into<String>) -> Self {
        self.proxy_ca_certificate = Some(proxy_ca_certificate.into());
        self
    }

    /// Set a list of hosts to exclude from proxy connections
    pub fn with_proxy_excludes(mut self, proxy_excludes: impl Into<String>) -> Self {
        self.proxy_excludes = Some(proxy_excludes.into());
        self
    }

    /// Set timeout for the overall request
    ///
    /// The timeout starts from when the request starts connecting until the
    /// response body has finished. If the request does not complete within the
    /// timeout, the client returns a timeout error.
    ///
    /// Timeout errors are retried, subject to the [`RetryConfig`]
    ///
    /// Default is 30 seconds
    ///
    /// # See Also
    /// * [`Self::with_timeout_disabled`] to disable the timeout
    /// * [`Self::with_connect_timeout`] to set a timeout for the connect phase
    ///
    /// [`RetryConfig`]: crate::RetryConfig
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(ConfigValue::Parsed(timeout));
        self
    }

    /// Disables the request timeout
    ///
    /// # See Also
    /// * [`Self::with_timeout`]
    pub fn with_timeout_disabled(mut self) -> Self {
        self.timeout = None;
        self
    }

    /// Set a timeout for only the connect phase of a Client
    ///
    /// This is the time allowed for the client to establish a connection
    /// and if the connection is not established within this time,
    /// the client returns a timeout error.
    ///
    /// Timeout errors are retried, subject to the [`RetryConfig`]
    ///
    /// Default is 5 seconds
    ///
    /// # See Also
    /// * [`Self::with_timeout`] to set a timeout for the overall request
    /// * [`Self::with_connect_timeout_disabled`] to disable the connect timeout
    ///
    /// [`RetryConfig`]: crate::RetryConfig
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(ConfigValue::Parsed(timeout));
        self
    }

    /// Disables the connection timeout
    ///
    /// # See Also
    /// * [`Self::with_connect_timeout`]
    pub fn with_connect_timeout_disabled(mut self) -> Self {
        self.connect_timeout = None;
        self
    }

    /// Set a read timeout
    ///
    /// The timeout applies to each read operation, and resets after a
    /// successful read. This is useful for detecting stalled connections
    /// when the size of the response is not known beforehand.
    ///
    /// Timeout errors are retried, subject to the [`RetryConfig`]
    ///
    /// Default is disabled (no read timeout)
    ///
    /// # See Also
    /// * [`Self::with_read_timeout_disabled`] to disable the read timeout
    /// * [`Self::with_timeout`] to set a timeout for the overall request
    /// * [`Self::with_connect_timeout`] to set a timeout for the connect phase
    ///
    /// [`RetryConfig`]: crate::RetryConfig
    pub fn with_read_timeout(mut self, timeout: Duration) -> Self {
        self.read_timeout = Some(ConfigValue::Parsed(timeout));
        self
    }

    /// Disables the read timeout
    ///
    /// # See Also
    /// * [`Self::with_read_timeout`]
    pub fn with_read_timeout_disabled(mut self) -> Self {
        self.read_timeout = None;
        self
    }

    /// Set the pool max idle timeout
    ///
    /// This is the length of time an idle connection will be kept alive
    ///
    /// Default is 90 seconds enforced by reqwest
    pub fn with_pool_idle_timeout(mut self, timeout: Duration) -> Self {
        self.pool_idle_timeout = Some(ConfigValue::Parsed(timeout));
        self
    }

    /// Set the maximum number of idle connections per host
    ///
    /// Default is no limit enforced by reqwest
    pub fn with_pool_max_idle_per_host(mut self, max: usize) -> Self {
        self.pool_max_idle_per_host = Some(max.into());
        self
    }

    /// Sets an interval for HTTP/2 Ping frames should be sent to keep a connection alive.
    ///
    /// Default is disabled enforced by reqwest
    pub fn with_http2_keep_alive_interval(mut self, interval: Duration) -> Self {
        self.http2_keep_alive_interval = Some(ConfigValue::Parsed(interval));
        self
    }

    /// Sets a timeout for receiving an acknowledgement of the keep-alive ping.
    ///
    /// If the ping is not acknowledged within the timeout, the connection will be closed.
    /// Does nothing if `http2_keep_alive_interval` is disabled.
    ///
    /// Default is disabled enforced by reqwest
    pub fn with_http2_keep_alive_timeout(mut self, interval: Duration) -> Self {
        self.http2_keep_alive_timeout = Some(ConfigValue::Parsed(interval));
        self
    }

    /// Enable HTTP/2 keep alive pings for idle connections
    ///
    /// If disabled, keep-alive pings are only sent while there are open request/response
    /// streams. If enabled, pings are also sent when no streams are active
    ///
    /// Default is disabled enforced by reqwest
    pub fn with_http2_keep_alive_while_idle(mut self) -> Self {
        self.http2_keep_alive_while_idle = true.into();
        self
    }

    /// Sets the maximum frame size to use for HTTP/2.
    ///
    /// Default is currently 16,384 but may change internally to optimize for common uses.
    pub fn with_http2_max_frame_size(mut self, sz: u32) -> Self {
        self.http2_max_frame_size = Some(ConfigValue::Parsed(sz));
        self
    }

    /// Override the default DNS resolution with a custom [`DnsResolver`]
    ///
    /// When set, [`ClientConfigKey::RandomizeAddresses`] is ignored: the
    /// provided resolver is fully responsible for resolution, ordering,
    /// shuffling, and caching.
    ///
    /// The built-in reqwest-based transport applies this automatically.
    /// Custom [`HttpConnector`] implementations should read it via
    /// [`Self::dns_resolver`] and honor it.
    ///
    /// Note: unlike other options, this cannot be configured via
    /// [`ClientConfigKey`] / string configuration.
    ///
    /// [`HttpConnector`]: crate::client::HttpConnector
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_dns_resolver(mut self, resolver: Arc<dyn DnsResolver>) -> Self {
        self.dns_resolver = Some(resolver);
        self
    }

    /// Return the custom [`DnsResolver`] if any, see [`Self::with_dns_resolver`]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn dns_resolver(&self) -> Option<&Arc<dyn DnsResolver>> {
        self.dns_resolver.as_ref()
    }

    /// Get the default headers defined through `ClientOptions::with_default_headers`
    pub fn get_default_headers(&self) -> Option<&HeaderMap> {
        self.default_headers.as_ref()
    }

    /// Get the mime type for the file in `path` to be uploaded
    ///
    /// Gets the file extension from `path`, and returns the
    /// mime type if it was defined initially through
    /// `ClientOptions::with_content_type_for_suffix`
    ///
    /// Otherwise, returns the default mime type if it was defined
    /// earlier through `ClientOptions::with_default_content_type`
    pub fn get_content_type(&self, path: &Path) -> Option<&str> {
        match path.extension() {
            Some(extension) => match self.content_type_map.get(extension) {
                Some(ct) => Some(ct.as_str()),
                None => self.default_content_type.as_deref(),
            },
            None => self.default_content_type.as_deref(),
        }
    }

    /// Returns a copy of this [`ClientOptions`] with overrides necessary for metadata endpoint access
    ///
    /// In particular:
    /// * Allows HTTP as metadata endpoints do not use TLS
    /// * Configures a low connection timeout to provide quick feedback if not present
    #[cfg(any(feature = "aws-base", feature = "gcp-base", feature = "azure-base"))]
    pub(crate) fn metadata_options(&self) -> Self {
        self.clone()
            .with_allow_http(true)
            .with_connect_timeout(Duration::from_secs(1))
    }

    #[cfg(all(feature = "reqwest", not(target_arch = "wasm32")))]
    pub(crate) fn client(&self) -> Result<reqwest::Client> {
        let mut builder = reqwest::ClientBuilder::new();

        match &self.user_agent {
            Some(user_agent) => builder = builder.user_agent(user_agent.get()?),
            None => builder = builder.user_agent(DEFAULT_USER_AGENT),
        }

        if let Some(headers) = &self.default_headers {
            builder = builder.default_headers(headers.clone())
        }

        if let Some(proxy) = &self.proxy_url {
            let mut proxy = Proxy::all(proxy).map_err(map_client_error)?;

            if let Some(certificate) = &self.proxy_ca_certificate {
                let certificate = reqwest::tls::Certificate::from_pem(certificate.as_bytes())
                    .map_err(map_client_error)?;

                builder = builder.tls_certs_merge(std::iter::once(certificate));
            }

            if let Some(proxy_excludes) = &self.proxy_excludes {
                let no_proxy = NoProxy::from_string(proxy_excludes);

                proxy = proxy.no_proxy(no_proxy);
            }

            builder = builder.proxy(proxy);
        }

        let certs = self
            .root_certificates
            .iter()
            .map(|certificate| certificate.0.clone());

        if self.no_system_certificates.get()? {
            builder = builder.tls_certs_only(certs);
        } else {
            builder = builder.tls_certs_merge(certs);
        }

        if let Some(timeout) = &self.timeout {
            builder = builder.timeout(timeout.get()?)
        }

        if let Some(timeout) = &self.connect_timeout {
            builder = builder.connect_timeout(timeout.get()?)
        }

        if let Some(timeout) = &self.read_timeout {
            builder = builder.read_timeout(timeout.get()?)
        }

        if let Some(timeout) = &self.pool_idle_timeout {
            builder = builder.pool_idle_timeout(timeout.get()?)
        }

        if let Some(max) = &self.pool_max_idle_per_host {
            builder = builder.pool_max_idle_per_host(max.get()?)
        }

        if let Some(interval) = &self.http2_keep_alive_interval {
            builder = builder.http2_keep_alive_interval(interval.get()?)
        }

        if let Some(interval) = &self.http2_keep_alive_timeout {
            builder = builder.http2_keep_alive_timeout(interval.get()?)
        }

        if self.http2_keep_alive_while_idle.get()? {
            builder = builder.http2_keep_alive_while_idle(true)
        }

        if let Some(sz) = &self.http2_max_frame_size {
            builder = builder.http2_max_frame_size(Some(sz.get()?))
        }

        if self.http1_only.get()? {
            builder = builder.http1_only()
        }

        if self.http2_only.get()? {
            builder = builder.http2_prior_knowledge()
        }

        if self.allow_invalid_certificates.get()? {
            builder = builder.danger_accept_invalid_certs(true)
        }

        // Explicitly disable compression, since it may be automatically enabled
        // when certain reqwest features are enabled. Compression interferes
        // with the `Content-Length` header, which is used to determine the
        // size of objects.
        builder = builder.no_gzip().no_brotli().no_zstd().no_deflate();

        let resolver: Option<Arc<dyn DnsResolver>> = match &self.dns_resolver {
            Some(resolver) => Some(Arc::clone(resolver)),
            None if self.randomize_addresses.get()? => Some(Arc::new(dns::ShuffleResolver)),
            None => None,
        };
        if let Some(resolver) = resolver {
            builder = builder.dns_resolver(Arc::new(dns::ReqwestResolver(resolver)));
        }

        builder
            .https_only(!self.allow_http.get()?)
            .build()
            .map_err(map_client_error)
    }

    #[cfg(all(feature = "reqwest", target_arch = "wasm32", target_os = "unknown"))]
    pub(crate) fn client(&self) -> Result<reqwest::Client> {
        let mut builder = reqwest::ClientBuilder::new();

        match &self.user_agent {
            Some(user_agent) => builder = builder.user_agent(user_agent.get()?),
            None => builder = builder.user_agent(DEFAULT_USER_AGENT),
        }

        if let Some(headers) = &self.default_headers {
            builder = builder.default_headers(headers.clone())
        }

        builder.build().map_err(map_client_error)
    }
}

pub(crate) trait GetOptionsExt {
    fn with_get_options(self, options: GetOptions) -> Self;
}

impl GetOptionsExt for HttpRequestBuilder {
    fn with_get_options(mut self, options: GetOptions) -> Self {
        use hyper::header::*;

        let GetOptions {
            if_match,
            if_none_match,
            if_modified_since,
            if_unmodified_since,
            range,
            version: _,
            head: _,
            extensions,
        } = options;

        if let Some(range) = range {
            self = self.header(RANGE, range.to_string());
        }

        if let Some(tag) = if_match {
            self = self.header(IF_MATCH, tag);
        }

        if let Some(tag) = if_none_match {
            self = self.header(IF_NONE_MATCH, tag);
        }

        const DATE_FORMAT: &str = "%a, %d %b %Y %H:%M:%S GMT";
        if let Some(date) = if_unmodified_since {
            self = self.header(IF_UNMODIFIED_SINCE, date.format(DATE_FORMAT).to_string());
        }

        if let Some(date) = if_modified_since {
            self = self.header(IF_MODIFIED_SINCE, date.format(DATE_FORMAT).to_string());
        }

        self = self.extensions(extensions);

        self
    }
}

/// Provides credentials for use when signing requests
#[async_trait]
pub trait CredentialProvider: std::fmt::Debug + Send + Sync {
    /// The type of credential returned by this provider
    type Credential;

    /// Return a credential
    async fn get_credential(&self) -> Result<Arc<Self::Credential>>;
}

/// A static set of credentials
#[derive(Debug)]
pub struct StaticCredentialProvider<T> {
    credential: Arc<T>,
}

impl<T> StaticCredentialProvider<T> {
    /// A [`CredentialProvider`] for a static credential of type `T`
    pub fn new(credential: T) -> Self {
        Self {
            credential: Arc::new(credential),
        }
    }
}

#[async_trait]
impl<T> CredentialProvider for StaticCredentialProvider<T>
where
    T: std::fmt::Debug + Send + Sync,
{
    type Credential = T;

    async fn get_credential(&self) -> Result<Arc<T>> {
        Ok(Arc::clone(&self.credential))
    }
}

#[cfg(any(feature = "aws-base", feature = "azure-base", feature = "gcp-base"))]
mod cloud {
    use super::*;
    use crate::RetryConfig;
    use crate::client::token::{TemporaryToken, TokenCache};

    /// A [`CredentialProvider`] that uses [`HttpClient`] to fetch temporary tokens
    #[derive(Debug)]
    pub(crate) struct TokenCredentialProvider<T: TokenProvider> {
        inner: T,
        client: HttpClient,
        retry: RetryConfig,
        cache: TokenCache<Arc<T::Credential>>,
    }

    impl<T: TokenProvider> TokenCredentialProvider<T> {
        pub(crate) fn new(inner: T, client: HttpClient, retry: RetryConfig) -> Self {
            Self {
                inner,
                client,
                retry,
                cache: Default::default(),
            }
        }

        /// Override the minimum remaining TTL for a cached token to be used
        #[cfg(any(feature = "aws-base", feature = "gcp-base"))]
        pub(crate) fn with_min_ttl(mut self, min_ttl: Duration) -> Self {
            self.cache = self.cache.with_min_ttl(min_ttl);
            self
        }
    }

    #[async_trait]
    impl<T: TokenProvider> CredentialProvider for TokenCredentialProvider<T> {
        type Credential = T::Credential;

        async fn get_credential(&self) -> Result<Arc<Self::Credential>> {
            self.cache
                .get_or_insert_with(|| self.inner.fetch_token(&self.client, &self.retry))
                .await
        }
    }

    #[async_trait]
    pub(crate) trait TokenProvider: std::fmt::Debug + Send + Sync {
        type Credential: std::fmt::Debug + Send + Sync;

        async fn fetch_token(
            &self,
            client: &HttpClient,
            retry: &RetryConfig,
        ) -> Result<TemporaryToken<Arc<Self::Credential>>>;
    }
}

use crate::client::builder::HttpRequestBuilder;
#[cfg(any(feature = "aws-base", feature = "azure-base", feature = "gcp-base"))]
pub(crate) use cloud::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn client_test_config_from_map() {
        let allow_http = "true".to_string();
        let allow_invalid_certificates = "false".to_string();
        let connect_timeout = "90 seconds".to_string();
        let default_content_type = "object_store:fake_default_content_type".to_string();
        let http1_only = "true".to_string();
        let http2_only = "false".to_string();
        let http2_keep_alive_interval = "90 seconds".to_string();
        let http2_keep_alive_timeout = "91 seconds".to_string();
        let http2_keep_alive_while_idle = "92 seconds".to_string();
        let http2_max_frame_size = "1337".to_string();
        let no_system_certificates = "true".to_string();
        let pool_idle_timeout = "93 seconds".to_string();
        let pool_max_idle_per_host = "94".to_string();
        let proxy_url = "https://fake_proxy_url".to_string();
        let read_timeout = "45 seconds".to_string();
        let timeout = "95 seconds".to_string();
        let user_agent = "object_store:fake_user_agent".to_string();

        let options = HashMap::from([
            ("allow_http", allow_http.clone()),
            (
                "allow_invalid_certificates",
                allow_invalid_certificates.clone(),
            ),
            ("connect_timeout", connect_timeout.clone()),
            ("default_content_type", default_content_type.clone()),
            ("http1_only", http1_only.clone()),
            ("http2_only", http2_only.clone()),
            (
                "http2_keep_alive_interval",
                http2_keep_alive_interval.clone(),
            ),
            ("http2_keep_alive_timeout", http2_keep_alive_timeout.clone()),
            (
                "http2_keep_alive_while_idle",
                http2_keep_alive_while_idle.clone(),
            ),
            ("http2_max_frame_size", http2_max_frame_size.clone()),
            (
                "disable_system_certificates",
                no_system_certificates.clone(),
            ),
            ("pool_idle_timeout", pool_idle_timeout.clone()),
            ("pool_max_idle_per_host", pool_max_idle_per_host.clone()),
            ("proxy_url", proxy_url.clone()),
            ("read_timeout", read_timeout.clone()),
            ("timeout", timeout.clone()),
            ("user_agent", user_agent.clone()),
        ]);

        let builder = options
            .into_iter()
            .fold(ClientOptions::new(), |builder, (key, value)| {
                builder.with_config(key.parse().unwrap(), value)
            });

        assert_eq!(
            builder
                .get_config_value(&ClientConfigKey::AllowHttp)
                .unwrap(),
            allow_http
        );
        assert_eq!(
            builder
                .get_config_value(&ClientConfigKey::AllowInvalidCertificates)
                .unwrap(),
            allow_invalid_certificates
        );
        assert_eq!(
            builder
                .get_config_value(&ClientConfigKey::ConnectTimeout)
                .unwrap(),
            connect_timeout
        );
        assert_eq!(
            builder
                .get_config_value(&ClientConfigKey::DefaultContentType)
                .unwrap(),
            default_content_type
        );
        assert_eq!(
            builder
                .get_config_value(&ClientConfigKey::Http1Only)
                .unwrap(),
            http1_only
        );
        assert_eq!(
            builder
                .get_config_value(&ClientConfigKey::Http2Only)
                .unwrap(),
            http2_only
        );
        assert_eq!(
            builder
                .get_config_value(&ClientConfigKey::Http2KeepAliveInterval)
                .unwrap(),
            http2_keep_alive_interval
        );
        assert_eq!(
            builder
                .get_config_value(&ClientConfigKey::Http2KeepAliveTimeout)
                .unwrap(),
            http2_keep_alive_timeout
        );
        assert_eq!(
            builder
                .get_config_value(&ClientConfigKey::Http2KeepAliveWhileIdle)
                .unwrap(),
            http2_keep_alive_while_idle
        );
        assert_eq!(
            builder
                .get_config_value(&ClientConfigKey::Http2MaxFrameSize)
                .unwrap(),
            http2_max_frame_size
        );
        assert_eq!(
            builder
                .get_config_value(&ClientConfigKey::NoSystemCertificates)
                .unwrap(),
            no_system_certificates
        );

        assert_eq!(
            builder
                .get_config_value(&ClientConfigKey::PoolIdleTimeout)
                .unwrap(),
            pool_idle_timeout
        );
        assert_eq!(
            builder
                .get_config_value(&ClientConfigKey::PoolMaxIdlePerHost)
                .unwrap(),
            pool_max_idle_per_host
        );
        assert_eq!(
            builder
                .get_config_value(&ClientConfigKey::ProxyUrl)
                .unwrap(),
            proxy_url
        );
        assert_eq!(
            builder
                .get_config_value(&ClientConfigKey::ReadTimeout)
                .unwrap(),
            read_timeout
        );
        assert_eq!(
            builder.get_config_value(&ClientConfigKey::Timeout).unwrap(),
            timeout
        );
        assert_eq!(
            builder
                .get_config_value(&ClientConfigKey::UserAgent)
                .unwrap(),
            user_agent
        );
    }

    #[tokio::test]
    #[cfg(all(feature = "reqwest", not(target_arch = "wasm32")))]
    async fn test_custom_dns_resolver() {
        use crate::client::mock_server::MockServer;
        use std::net::{IpAddr, Ipv4Addr};
        use std::sync::atomic::{AtomicUsize, Ordering};

        #[derive(Debug, Default)]
        struct CountingResolver(AtomicUsize);

        impl DnsResolver for CountingResolver {
            fn resolve(&self, host: &str) -> DnsFuture {
                assert_eq!(host, "localhost");
                self.0.fetch_add(1, Ordering::SeqCst);
                Box::pin(async { Ok(vec![IpAddr::V4(Ipv4Addr::LOCALHOST)]) })
            }
        }

        let server = MockServer::new().await;
        server.push(::http::Response::new("hello".to_string()));

        let resolver = Arc::new(CountingResolver::default());
        let client = ClientOptions::new()
            .with_allow_http(true)
            .with_dns_resolver(Arc::clone(&resolver) as Arc<dyn DnsResolver>)
            .client()
            .unwrap();

        // Use a hostname (not an IP literal) so resolution actually runs
        let parsed = url::Url::parse(server.url()).unwrap();
        let url = format!("http://localhost:{}/", parsed.port().unwrap());
        let resp = client.get(url).send().await.unwrap();
        assert!(resp.status().is_success());
        assert_eq!(resolver.0.load(Ordering::SeqCst), 1);
    }
}
