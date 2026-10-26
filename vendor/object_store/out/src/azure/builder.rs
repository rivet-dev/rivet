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

use crate::azure::client::{AzureClient, AzureConfig, AzureEncryptionHeaders};
use crate::azure::credential::{
    AzureAccessKey, AzureCliCredential, ClientSecretOAuthProvider, FabricTokenOAuthProvider,
    ImdsManagedIdentityProvider, WorkloadIdentityOAuthProvider,
};
use crate::azure::{AzureCredential, AzureCredentialProvider, MicrosoftAzure, STORE};
use crate::client::{CryptoProvider, HttpConnector, TokenCredentialProvider, http_connector};
use crate::config::ConfigValue;
use crate::{ClientConfigKey, ClientOptions, Result, RetryConfig, StaticCredentialProvider};
use percent_encoding::percent_decode_str;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use url::Url;

/// The well-known account used by Azurite and the legacy Azure Storage Emulator.
///
/// <https://docs.microsoft.com/azure/storage/common/storage-use-azurite#well-known-storage-account-and-key>
const EMULATOR_ACCOUNT: &str = "devstoreaccount1";

/// The well-known account key used by Azurite and the legacy Azure Storage Emulator.
///
/// <https://docs.microsoft.com/azure/storage/common/storage-use-azurite#well-known-storage-account-and-key>
const EMULATOR_ACCOUNT_KEY: &str =
    "Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw==";

const MSI_ENDPOINT_ENV_KEY: &str = "IDENTITY_ENDPOINT";

/// A specialized `Error` for Azure builder-related errors
#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Unable parse source url. Url: {}, Error: {}", url, source)]
    UnableToParseUrl {
        source: url::ParseError,
        url: String,
    },

    #[error(
        "Unable parse emulator url {}={}, Error: {}",
        env_name,
        env_value,
        source
    )]
    UnableToParseEmulatorUrl {
        env_name: String,
        env_value: String,
        source: url::ParseError,
    },

    #[error("Account must be specified")]
    MissingAccount {},

    #[error("Container name must be specified")]
    MissingContainerName {},

    #[error(
        "Unknown url scheme cannot be parsed into storage location: {}",
        scheme
    )]
    UnknownUrlScheme { scheme: String },

    #[error("URL did not match any known pattern for scheme: {}", url)]
    UrlNotRecognised { url: String },

    #[error("Failed parsing an SAS key")]
    DecodeSasKey { source: std::str::Utf8Error },

    #[error("Missing component in SAS query pair")]
    MissingSasComponent {},

    #[error("Invalid encryption key: {source}")]
    InvalidEncryptionKey {
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    #[error("Configuration key: '{}' is not known.", key)]
    UnknownConfigurationKey { key: String },

    #[error(
        "Unknown credential type: '{}'. Supported values: auto, bearer_token, access_key, client_secret, workload_identity, sas_token, azure_cli, managed_identity",
        credential_type
    )]
    UnknownCredentialType { credential_type: String },

    #[error(
        "Credential type '{}' was requested but required configuration is missing",
        credential_type
    )]
    MissingCredentialConfig { credential_type: String },
}

impl From<Error> for crate::Error {
    fn from(source: Error) -> Self {
        match source {
            Error::UnknownConfigurationKey { key } => {
                Self::UnknownConfigurationKey { store: STORE, key }
            }
            _ => Self::Generic {
                store: STORE,
                source: Box::new(source),
            },
        }
    }
}

/// Configure a connection to Microsoft Azure Blob Storage container using
/// the specified credentials.
///
/// # Example
/// ```
/// # let ACCOUNT = "foo";
/// # let BUCKET_NAME = "foo";
/// # let ACCESS_KEY = "foo";
/// # use object_store::azure::MicrosoftAzureBuilder;
/// let azure = MicrosoftAzureBuilder::new()
///  .with_account(ACCOUNT)
///  .with_access_key(ACCESS_KEY)
///  .with_container_name(BUCKET_NAME)
///  .build();
/// ```
#[derive(Default, Clone)]
pub struct MicrosoftAzureBuilder {
    /// Account name
    account_name: Option<String>,
    /// Access key
    access_key: Option<String>,
    /// Container name
    container_name: Option<String>,
    /// Bearer token
    bearer_token: Option<String>,
    /// Client id
    client_id: Option<String>,
    /// Client secret
    client_secret: Option<String>,
    /// Tenant id
    tenant_id: Option<String>,
    /// Query pairs for shared access signature authorization
    sas_query_pairs: Option<Vec<(String, String)>>,
    /// Shared access signature
    sas_key: Option<String>,
    /// Authority host
    authority_host: Option<String>,
    /// Url
    url: Option<String>,
    /// When set to true, azurite storage emulator has to be used
    use_emulator: ConfigValue<bool>,
    /// Storage endpoint
    endpoint: Option<String>,
    /// Msi endpoint for acquiring managed identity token
    msi_endpoint: Option<String>,
    /// Object id for use with managed identity authentication
    object_id: Option<String>,
    /// Msi resource id for use with managed identity authentication
    msi_resource_id: Option<String>,
    /// File containing token for Azure AD workload identity federation
    federated_token_file: Option<String>,
    /// When set to true, azure cli has to be used for acquiring access token
    use_azure_cli: ConfigValue<bool>,
    /// Retry config
    retry_config: RetryConfig,
    /// Client options
    client_options: ClientOptions,
    /// Credentials
    credentials: Option<AzureCredentialProvider>,
    /// The [`CryptoProvider`] to use
    crypto: Option<Arc<dyn CryptoProvider>>,
    /// Skip signing requests
    skip_signature: ConfigValue<bool>,
    /// When set to true, fabric url scheme will be used
    ///
    /// i.e. https://{account_name}.dfs.fabric.microsoft.com
    use_fabric_endpoint: ConfigValue<bool>,
    /// When set to true, skips tagging objects
    disable_tagging: ConfigValue<bool>,
    /// Fabric token service url
    fabric_token_service_url: Option<String>,
    /// Fabric workload host
    fabric_workload_host: Option<String>,
    /// Fabric session token
    fabric_session_token: Option<String>,
    /// Fabric cluster identifier
    fabric_cluster_identifier: Option<String>,
    /// Credential type override
    credential_type: Option<String>,
    /// Base64-encoded 256-bit customer-provided encryption key
    encryption_key: Option<String>,
    /// The [`HttpConnector`] to use
    http_connector: Option<Arc<dyn HttpConnector>>,
}

/// Configuration keys for [`MicrosoftAzureBuilder`]
///
/// Configuration via keys can be done via [`MicrosoftAzureBuilder::with_config`]
///
/// # Example
/// ```
/// # use object_store::azure::{MicrosoftAzureBuilder, AzureConfigKey};
/// let builder = MicrosoftAzureBuilder::new()
///     .with_config("azure_client_id".parse().unwrap(), "my-client-id")
///     .with_config(AzureConfigKey::AuthorityId, "my-tenant-id");
/// ```
#[derive(PartialEq, Eq, Hash, Clone, Debug, Copy, Deserialize, Serialize)]
#[non_exhaustive]
pub enum AzureConfigKey {
    /// The name of the azure storage account
    ///
    /// Supported keys:
    /// - `azure_storage_account_name`
    /// - `account_name`
    AccountName,

    /// Master key for accessing storage account
    ///
    /// Supported keys:
    /// - `azure_storage_account_key`
    /// - `azure_storage_access_key`
    /// - `azure_storage_master_key`
    /// - `access_key`
    /// - `account_key`
    /// - `master_key`
    AccessKey,

    /// Service principal client id for authorizing requests
    ///
    /// Supported keys:
    /// - `azure_storage_client_id`
    /// - `azure_client_id`
    /// - `client_id`
    ClientId,

    /// Service principal client secret for authorizing requests
    ///
    /// Supported keys:
    /// - `azure_storage_client_secret`
    /// - `azure_client_secret`
    /// - `client_secret`
    ClientSecret,

    /// Tenant id used in oauth flows
    ///
    /// Supported keys:
    /// - `azure_storage_tenant_id`
    /// - `azure_storage_authority_id`
    /// - `azure_tenant_id`
    /// - `azure_authority_id`
    /// - `tenant_id`
    /// - `authority_id`
    AuthorityId,

    /// Authority host used in oauth flows
    ///
    /// Supported keys:
    /// - `azure_storage_authority_host`
    /// - `azure_authority_host`
    /// - `authority_host`
    AuthorityHost,

    /// Shared access signature.
    ///
    /// The signature is expected to be percent-encoded, much like they are provided
    /// in the azure storage explorer or azure portal.
    ///
    /// Supported keys:
    /// - `azure_storage_sas_key`
    /// - `azure_storage_sas_token`
    /// - `sas_key`
    /// - `sas_token`
    SasKey,

    /// Bearer token
    ///
    /// Supported keys:
    /// - `azure_storage_token`
    /// - `bearer_token`
    /// - `token`
    Token,

    /// Use object store with azurite storage emulator
    ///
    /// Supported keys:
    /// - `azure_storage_use_emulator`
    /// - `object_store_use_emulator`
    /// - `use_emulator`
    UseEmulator,

    /// Override the endpoint used to communicate with blob storage
    ///
    /// Supported keys:
    /// - `azure_storage_endpoint`
    /// - `azure_endpoint`
    /// - `endpoint`
    Endpoint,

    /// Use object store with url scheme account.dfs.fabric.microsoft.com
    ///
    /// Supported keys:
    /// - `azure_use_fabric_endpoint`
    /// - `use_fabric_endpoint`
    UseFabricEndpoint,

    /// Endpoint to request a imds managed identity token
    ///
    /// Supported keys:
    /// - `azure_msi_endpoint`
    /// - `azure_identity_endpoint`
    /// - `identity_endpoint`
    /// - `msi_endpoint`
    MsiEndpoint,

    /// Object id for use with managed identity authentication
    ///
    /// Supported keys:
    /// - `azure_object_id`
    /// - `object_id`
    ObjectId,

    /// Msi resource id for use with managed identity authentication
    ///
    /// Supported keys:
    /// - `azure_msi_resource_id`
    /// - `msi_resource_id`
    MsiResourceId,

    /// File containing token for Azure AD workload identity federation
    ///
    /// Supported keys:
    /// - `azure_federated_token_file`
    /// - `federated_token_file`
    FederatedTokenFile,

    /// Use azure cli for acquiring access token
    ///
    /// Supported keys:
    /// - `azure_use_azure_cli`
    /// - `use_azure_cli`
    UseAzureCli,

    /// Skip signing requests
    ///
    /// Supported keys:
    /// - `azure_skip_signature`
    /// - `skip_signature`
    SkipSignature,

    /// Container name
    ///
    /// Supported keys:
    /// - `azure_container_name`
    /// - `container_name`
    ContainerName,

    /// Disables tagging objects
    ///
    /// This can be desirable if not supported by the backing store
    ///
    /// Supported keys:
    /// - `azure_disable_tagging`
    /// - `disable_tagging`
    DisableTagging,

    /// Fabric token service url
    ///
    /// Supported keys:
    /// - `azure_fabric_token_service_url`
    /// - `fabric_token_service_url`
    FabricTokenServiceUrl,

    /// Fabric workload host
    ///
    /// Supported keys:
    /// - `azure_fabric_workload_host`
    /// - `fabric_workload_host`
    FabricWorkloadHost,

    /// Fabric session token
    ///
    /// Supported keys:
    /// - `azure_fabric_session_token`
    /// - `fabric_session_token`
    FabricSessionToken,

    /// Fabric cluster identifier
    ///
    /// Supported keys:
    /// - `azure_fabric_cluster_identifier`
    /// - `fabric_cluster_identifier`
    FabricClusterIdentifier,

    /// Credential type to use for authentication
    ///
    /// When multiple credential configurations are present, this key forces
    /// the builder to use a specific credential type instead of relying on
    /// the default resolution order.
    ///
    /// Supported values:
    /// - `auto` (default) — use the built-in priority chain
    /// - `bearer_token` — use a static bearer token
    /// - `access_key` — use an access key
    /// - `client_secret` — use client secret (service principal) OAuth
    /// - `workload_identity` — use workload identity federation
    /// - `sas_token` — use a shared access signature
    /// - `azure_cli` — use Azure CLI
    /// - `managed_identity` — use IMDS managed identity
    ///
    /// Supported keys:
    /// - `azure_credential_type`
    /// - `credential_type`
    CredentialType,

    /// Base64-encoded customer-provided encryption key
    ///
    /// Supported keys:
    /// - `azure_storage_encryption_key`
    /// - `encryption_key`
    EncryptionKey,

    /// Client options
    Client(ClientConfigKey),
}

impl AsRef<str> for AzureConfigKey {
    fn as_ref(&self) -> &str {
        match self {
            Self::AccountName => "azure_storage_account_name",
            Self::AccessKey => "azure_storage_account_key",
            Self::ClientId => "azure_storage_client_id",
            Self::ClientSecret => "azure_storage_client_secret",
            Self::AuthorityId => "azure_storage_tenant_id",
            Self::AuthorityHost => "azure_storage_authority_host",
            Self::SasKey => "azure_storage_sas_key",
            Self::Token => "azure_storage_token",
            Self::UseEmulator => "azure_storage_use_emulator",
            Self::UseFabricEndpoint => "azure_use_fabric_endpoint",
            Self::Endpoint => "azure_storage_endpoint",
            Self::MsiEndpoint => "azure_msi_endpoint",
            Self::ObjectId => "azure_object_id",
            Self::MsiResourceId => "azure_msi_resource_id",
            Self::FederatedTokenFile => "azure_federated_token_file",
            Self::UseAzureCli => "azure_use_azure_cli",
            Self::SkipSignature => "azure_skip_signature",
            Self::ContainerName => "azure_container_name",
            Self::DisableTagging => "azure_disable_tagging",
            Self::FabricTokenServiceUrl => "azure_fabric_token_service_url",
            Self::FabricWorkloadHost => "azure_fabric_workload_host",
            Self::FabricSessionToken => "azure_fabric_session_token",
            Self::FabricClusterIdentifier => "azure_fabric_cluster_identifier",
            Self::CredentialType => "azure_credential_type",
            Self::EncryptionKey => "azure_storage_encryption_key",
            Self::Client(key) => key.as_ref(),
        }
    }
}

impl FromStr for AzureConfigKey {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "azure_storage_account_key"
            | "azure_storage_access_key"
            | "azure_storage_master_key"
            | "master_key"
            | "account_key"
            | "access_key" => Ok(Self::AccessKey),
            "azure_storage_account_name" | "account_name" => Ok(Self::AccountName),
            "azure_storage_client_id" | "azure_client_id" | "client_id" => Ok(Self::ClientId),
            "azure_storage_client_secret" | "azure_client_secret" | "client_secret" => {
                Ok(Self::ClientSecret)
            }
            "azure_storage_tenant_id"
            | "azure_storage_authority_id"
            | "azure_tenant_id"
            | "azure_authority_id"
            | "tenant_id"
            | "authority_id" => Ok(Self::AuthorityId),
            "azure_storage_authority_host" | "azure_authority_host" | "authority_host" => {
                Ok(Self::AuthorityHost)
            }
            "azure_storage_sas_key" | "azure_storage_sas_token" | "sas_key" | "sas_token" => {
                Ok(Self::SasKey)
            }
            "azure_storage_token" | "bearer_token" | "token" => Ok(Self::Token),
            "azure_storage_use_emulator" | "use_emulator" => Ok(Self::UseEmulator),
            "azure_storage_endpoint" | "azure_endpoint" | "endpoint" => Ok(Self::Endpoint),
            "azure_msi_endpoint"
            | "azure_identity_endpoint"
            | "identity_endpoint"
            | "msi_endpoint" => Ok(Self::MsiEndpoint),
            "azure_object_id" | "object_id" => Ok(Self::ObjectId),
            "azure_msi_resource_id" | "msi_resource_id" => Ok(Self::MsiResourceId),
            "azure_federated_token_file" | "federated_token_file" => Ok(Self::FederatedTokenFile),
            "azure_use_fabric_endpoint" | "use_fabric_endpoint" => Ok(Self::UseFabricEndpoint),
            "azure_use_azure_cli" | "use_azure_cli" => Ok(Self::UseAzureCli),
            "azure_skip_signature" | "skip_signature" => Ok(Self::SkipSignature),
            "azure_container_name" | "container_name" => Ok(Self::ContainerName),
            "azure_disable_tagging" | "disable_tagging" => Ok(Self::DisableTagging),
            "azure_fabric_token_service_url" | "fabric_token_service_url" => {
                Ok(Self::FabricTokenServiceUrl)
            }
            "azure_fabric_workload_host" | "fabric_workload_host" => Ok(Self::FabricWorkloadHost),
            "azure_fabric_session_token" | "fabric_session_token" => Ok(Self::FabricSessionToken),
            "azure_fabric_cluster_identifier" | "fabric_cluster_identifier" => {
                Ok(Self::FabricClusterIdentifier)
            }
            "azure_credential_type" | "credential_type" => Ok(Self::CredentialType),
            "azure_storage_encryption_key" | "encryption_key" => Ok(Self::EncryptionKey),
            // Backwards compatibility
            "azure_allow_http" => Ok(Self::Client(ClientConfigKey::AllowHttp)),
            _ => match s.strip_prefix("azure_").unwrap_or(s).parse() {
                Ok(key) => Ok(Self::Client(key)),
                Err(_) => Err(Error::UnknownConfigurationKey { key: s.into() }.into()),
            },
        }
    }
}

impl std::fmt::Debug for MicrosoftAzureBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MicrosoftAzureBuilder {{ account: {:?}, container_name: {:?} }}",
            self.account_name, self.container_name
        )
    }
}

impl MicrosoftAzureBuilder {
    /// Create a new [`MicrosoftAzureBuilder`] with default values.
    pub fn new() -> Self {
        Default::default()
    }

    /// Create an instance of [`MicrosoftAzureBuilder`] with values pre-populated from environment variables.
    ///
    /// Variables extracted from environment:
    /// * AZURE_STORAGE_ACCOUNT_NAME: storage account name
    /// * AZURE_STORAGE_ACCOUNT_KEY: storage account master key
    /// * AZURE_STORAGE_ACCESS_KEY: alias for AZURE_STORAGE_ACCOUNT_KEY
    /// * AZURE_STORAGE_CLIENT_ID -> client id for service principal authorization
    /// * AZURE_STORAGE_CLIENT_SECRET -> client secret for service principal authorization
    /// * AZURE_STORAGE_TENANT_ID -> tenant id used in oauth flows
    /// # Example
    /// ```
    /// use object_store::azure::MicrosoftAzureBuilder;
    ///
    /// let azure = MicrosoftAzureBuilder::from_env()
    ///     .with_container_name("foo")
    ///     .build();
    /// ```
    pub fn from_env() -> Self {
        let mut builder = Self::default();
        for (os_key, os_value) in std::env::vars_os() {
            if let (Some(key), Some(value)) = (os_key.to_str(), os_value.to_str()) {
                if key.starts_with("AZURE_") {
                    if let Ok(config_key) = key.to_ascii_lowercase().parse() {
                        builder = builder.with_config(config_key, value);
                    }
                }
            }
        }

        if let Ok(text) = std::env::var(MSI_ENDPOINT_ENV_KEY) {
            builder = builder.with_msi_endpoint(text);
        }

        builder
    }

    /// Parse available connection info form a well-known storage URL.
    ///
    /// The supported url schemes are:
    ///
    /// - `abfs[s]://<container>/<path>` (according to [fsspec](https://github.com/fsspec/adlfs))
    /// - `abfs[s]://<file_system>@<account_name>.dfs.core.windows.net/<path>`
    /// - `abfs[s]://<file_system>@<account_name>.dfs.fabric.microsoft.com/<path>`
    /// - `az://<container>/<path>` (according to [fsspec](https://github.com/fsspec/adlfs))
    /// - `adl://<container>/<path>` (according to [fsspec](https://github.com/fsspec/adlfs))
    /// - `azure://<container>/<path>` (custom)
    /// - `https://<account>.dfs.core.windows.net`
    /// - `https://<account>.blob.core.windows.net`
    /// - `https://<account>.blob.core.windows.net/<container>`
    /// - `https://<account>.dfs.fabric.microsoft.com`
    /// - `https://<account>.dfs.fabric.microsoft.com/<container>`
    /// - `https://<account>.blob.fabric.microsoft.com`
    /// - `https://<account>.blob.fabric.microsoft.com/<container>`
    ///
    /// Note: Settings derived from the URL will override any others set on this builder
    ///
    /// # Example
    /// ```
    /// use object_store::azure::MicrosoftAzureBuilder;
    ///
    /// let azure = MicrosoftAzureBuilder::from_env()
    ///     .with_url("abfss://file_system@account.dfs.core.windows.net/")
    ///     .build();
    /// ```
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Set an option on the builder via a key - value pair.
    pub fn with_config(mut self, key: AzureConfigKey, value: impl Into<String>) -> Self {
        match key {
            AzureConfigKey::AccessKey => self.access_key = Some(value.into()),
            AzureConfigKey::AccountName => self.account_name = Some(value.into()),
            AzureConfigKey::ClientId => self.client_id = Some(value.into()),
            AzureConfigKey::ClientSecret => self.client_secret = Some(value.into()),
            AzureConfigKey::AuthorityId => self.tenant_id = Some(value.into()),
            AzureConfigKey::AuthorityHost => self.authority_host = Some(value.into()),
            AzureConfigKey::SasKey => self.sas_key = Some(value.into()),
            AzureConfigKey::Token => self.bearer_token = Some(value.into()),
            AzureConfigKey::MsiEndpoint => self.msi_endpoint = Some(value.into()),
            AzureConfigKey::ObjectId => self.object_id = Some(value.into()),
            AzureConfigKey::MsiResourceId => self.msi_resource_id = Some(value.into()),
            AzureConfigKey::FederatedTokenFile => self.federated_token_file = Some(value.into()),
            AzureConfigKey::UseAzureCli => self.use_azure_cli.parse(value),
            AzureConfigKey::SkipSignature => self.skip_signature.parse(value),
            AzureConfigKey::UseEmulator => self.use_emulator.parse(value),
            AzureConfigKey::Endpoint => self.endpoint = Some(value.into()),
            AzureConfigKey::UseFabricEndpoint => self.use_fabric_endpoint.parse(value),
            AzureConfigKey::Client(key) => {
                self.client_options = self.client_options.with_config(key, value)
            }
            AzureConfigKey::ContainerName => self.container_name = Some(value.into()),
            AzureConfigKey::DisableTagging => self.disable_tagging.parse(value),
            AzureConfigKey::FabricTokenServiceUrl => {
                self.fabric_token_service_url = Some(value.into())
            }
            AzureConfigKey::FabricWorkloadHost => self.fabric_workload_host = Some(value.into()),
            AzureConfigKey::FabricSessionToken => self.fabric_session_token = Some(value.into()),
            AzureConfigKey::FabricClusterIdentifier => {
                self.fabric_cluster_identifier = Some(value.into())
            }
            AzureConfigKey::CredentialType => self.credential_type = Some(value.into()),
            AzureConfigKey::EncryptionKey => self.encryption_key = Some(value.into()),
        };
        self
    }

    /// Get config value via a [`AzureConfigKey`].
    ///
    /// # Example
    /// ```
    /// use object_store::azure::{MicrosoftAzureBuilder, AzureConfigKey};
    ///
    /// let builder = MicrosoftAzureBuilder::from_env()
    ///     .with_account("foo");
    /// let account_name = builder.get_config_value(&AzureConfigKey::AccountName).unwrap_or_default();
    /// assert_eq!("foo", &account_name);
    /// ```
    pub fn get_config_value(&self, key: &AzureConfigKey) -> Option<String> {
        match key {
            AzureConfigKey::AccountName => self.account_name.clone(),
            AzureConfigKey::AccessKey => self.access_key.clone(),
            AzureConfigKey::ClientId => self.client_id.clone(),
            AzureConfigKey::ClientSecret => self.client_secret.clone(),
            AzureConfigKey::AuthorityId => self.tenant_id.clone(),
            AzureConfigKey::AuthorityHost => self.authority_host.clone(),
            AzureConfigKey::SasKey => self.sas_key.clone(),
            AzureConfigKey::Token => self.bearer_token.clone(),
            AzureConfigKey::UseEmulator => Some(self.use_emulator.to_string()),
            AzureConfigKey::UseFabricEndpoint => Some(self.use_fabric_endpoint.to_string()),
            AzureConfigKey::Endpoint => self.endpoint.clone(),
            AzureConfigKey::MsiEndpoint => self.msi_endpoint.clone(),
            AzureConfigKey::ObjectId => self.object_id.clone(),
            AzureConfigKey::MsiResourceId => self.msi_resource_id.clone(),
            AzureConfigKey::FederatedTokenFile => self.federated_token_file.clone(),
            AzureConfigKey::UseAzureCli => Some(self.use_azure_cli.to_string()),
            AzureConfigKey::SkipSignature => Some(self.skip_signature.to_string()),
            AzureConfigKey::Client(key) => self.client_options.get_config_value(key),
            AzureConfigKey::ContainerName => self.container_name.clone(),
            AzureConfigKey::DisableTagging => Some(self.disable_tagging.to_string()),
            AzureConfigKey::FabricTokenServiceUrl => self.fabric_token_service_url.clone(),
            AzureConfigKey::FabricWorkloadHost => self.fabric_workload_host.clone(),
            AzureConfigKey::FabricSessionToken => self.fabric_session_token.clone(),
            AzureConfigKey::FabricClusterIdentifier => self.fabric_cluster_identifier.clone(),
            AzureConfigKey::CredentialType => self.credential_type.clone(),
            AzureConfigKey::EncryptionKey => self.encryption_key.clone(),
        }
    }

    /// Sets properties on this builder based on a URL
    ///
    /// This is a separate member function to allow fallible computation to
    /// be deferred until [`Self::build`] which in turn allows deriving [`Clone`]
    fn parse_url(&mut self, url: &str) -> Result<()> {
        let parsed = Url::parse(url).map_err(|source| {
            let url = url.into();
            Error::UnableToParseUrl { url, source }
        })?;

        let host = parsed
            .host_str()
            .ok_or_else(|| Error::UrlNotRecognised { url: url.into() })?;

        let validate = |s: &str| match s.contains('.') {
            true => Err(Error::UrlNotRecognised { url: url.into() }),
            false => Ok(s.to_string()),
        };

        match parsed.scheme() {
            "adl" | "azure" => self.container_name = Some(validate(host)?),
            "az" | "abfs" | "abfss" => {
                // abfs(s) might refer to the fsspec convention abfs://<container>/<path>
                // or the convention for the hadoop driver abfs[s]://<file_system>@<account_name>.dfs.core.windows.net/<path>
                if parsed.username().is_empty() {
                    self.container_name = Some(validate(host)?);
                } else {
                    match host.split_once('.') {
                        // Workspace-level Private Link detection
                        // "{workspaceid}.z??.(onelake|dfs|blob).fabric.microsoft.com"
                        Some((workspaceid, rest))
                            if rest.starts_with('z') && rest.ends_with("fabric.microsoft.com") =>
                        {
                            // Account name for WS-PL is two labels: "{workspaceid}.z{xy}"
                            let (zone, _) = rest.split_once('.').unwrap_or((rest, ""));

                            self.account_name = Some(format!("{workspaceid}.{zone}"));
                            self.endpoint = Some(format!("https://{}", host));

                            self.container_name = Some(validate(parsed.username())?);
                            self.use_fabric_endpoint = true.into();
                        }

                        Some((a, "dfs.core.windows.net")) | Some((a, "blob.core.windows.net")) => {
                            self.account_name = Some(validate(a)?);
                            self.container_name = Some(validate(parsed.username())?);
                        }

                        Some((a, "dfs.fabric.microsoft.com"))
                        | Some((a, "blob.fabric.microsoft.com")) => {
                            self.account_name = Some(validate(a)?);
                            self.container_name = Some(validate(parsed.username())?);
                            self.use_fabric_endpoint = true.into();
                        }
                        _ => return Err(Error::UrlNotRecognised { url: url.into() }.into()),
                    }
                }
            }
            "https" => match host.split_once('.') {
                // Workspace-level Private Link detection
                // "{workspaceid}.z??.(onelake|dfs|blob).fabric.microsoft.com"
                Some((workspaceid, rest))
                    if rest.starts_with('z') && rest.ends_with("fabric.microsoft.com") =>
                {
                    // rest looks like: "z28.dfs.fabric.microsoft.com" / "z28.blob.fabric.microsoft.com" / etc.
                    // Account name for WS-PL is two labels: "{workspaceid}.z{xy}"
                    let (zone, _) = rest.split_once('.').unwrap_or((rest, ""));

                    self.account_name = Some(format!("{workspaceid}.{zone}"));
                    self.endpoint = Some(format!("https://{}", host));

                    // Attempt to infer the container name from the URL
                    let container = parsed.path_segments().unwrap().next().expect(
                        "iterator always contains at least one string (which may be empty)",
                    );

                    if !container.is_empty() {
                        self.container_name = Some(validate(container)?);
                    }

                    self.use_fabric_endpoint = true.into();
                }

                Some((a, "dfs.core.windows.net")) | Some((a, "blob.core.windows.net")) => {
                    self.account_name = Some(validate(a)?);
                    let container = parsed.path_segments().unwrap().next().expect(
                        "iterator always contains at least one string (which may be empty)",
                    );
                    if !container.is_empty() {
                        self.container_name = Some(validate(container)?);
                    }
                }
                Some((a, "dfs.fabric.microsoft.com")) | Some((a, "blob.fabric.microsoft.com")) => {
                    self.account_name = Some(validate(a)?);
                    // Attempt to infer the container name from the URL
                    // - https://onelake.dfs.fabric.microsoft.com/<workspaceGUID>/<itemGUID>/Files/test.csv
                    // - https://onelake.dfs.fabric.microsoft.com/<workspace>/<item>.<itemtype>/<path>/<fileName>
                    //
                    // See <https://learn.microsoft.com/en-us/fabric/onelake/onelake-access-api>
                    let workspace = parsed.path_segments().unwrap().next().expect(
                        "iterator always contains at least one string (which may be empty)",
                    );
                    if !workspace.is_empty() {
                        self.container_name = Some(workspace.to_string())
                    }
                    self.use_fabric_endpoint = true.into();
                }
                _ => return Err(Error::UrlNotRecognised { url: url.into() }.into()),
            },
            scheme => {
                let scheme = scheme.into();
                return Err(Error::UnknownUrlScheme { scheme }.into());
            }
        }
        Ok(())
    }

    /// Set the Azure Account (required)
    pub fn with_account(mut self, account: impl Into<String>) -> Self {
        self.account_name = Some(account.into());
        self
    }

    /// Set the Azure Container Name (required)
    pub fn with_container_name(mut self, container_name: impl Into<String>) -> Self {
        self.container_name = Some(container_name.into());
        self
    }

    /// Set the Azure Access Key (required - one of access key, bearer token, or client credentials)
    pub fn with_access_key(mut self, access_key: impl Into<String>) -> Self {
        self.access_key = Some(access_key.into());
        self
    }

    /// Set a static bearer token to be used for authorizing requests
    pub fn with_bearer_token_authorization(mut self, bearer_token: impl Into<String>) -> Self {
        self.bearer_token = Some(bearer_token.into());
        self
    }

    /// Set a client secret used for client secret authorization
    pub fn with_client_secret_authorization(
        mut self,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        tenant_id: impl Into<String>,
    ) -> Self {
        self.client_id = Some(client_id.into());
        self.client_secret = Some(client_secret.into());
        self.tenant_id = Some(tenant_id.into());
        self
    }

    /// Sets the client id for use in client secret or k8s federated credential flow
    pub fn with_client_id(mut self, client_id: impl Into<String>) -> Self {
        self.client_id = Some(client_id.into());
        self
    }

    /// Sets the client secret for use in client secret flow
    pub fn with_client_secret(mut self, client_secret: impl Into<String>) -> Self {
        self.client_secret = Some(client_secret.into());
        self
    }

    /// Sets the tenant id for use in client secret or k8s federated credential flow
    pub fn with_tenant_id(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    /// Set query pairs appended to the url for shared access signature authorization
    pub fn with_sas_authorization(mut self, query_pairs: impl Into<Vec<(String, String)>>) -> Self {
        self.sas_query_pairs = Some(query_pairs.into());
        self
    }

    /// Set the credential provider overriding any other options
    pub fn with_credentials(mut self, credentials: AzureCredentialProvider) -> Self {
        self.credentials = Some(credentials);
        self
    }

    /// The [`CryptoProvider`] to use
    pub fn with_crypto_provider(mut self, provider: Arc<dyn CryptoProvider>) -> Self {
        self.crypto = Some(provider);
        self
    }

    /// Set if the Azure emulator should be used (defaults to false)
    pub fn with_use_emulator(mut self, use_emulator: bool) -> Self {
        self.use_emulator = use_emulator.into();
        self
    }

    /// Override the endpoint used to communicate with blob storage
    ///
    /// Defaults to `https://{account}.blob.core.windows.net`
    ///
    /// By default, only HTTPS schemes are enabled. To connect to an HTTP endpoint, enable
    /// [`Self::with_allow_http`].
    pub fn with_endpoint(mut self, endpoint: String) -> Self {
        self.endpoint = Some(endpoint);
        self
    }

    /// Set if Microsoft Fabric url scheme should be used (defaults to false)
    ///
    /// When disabled the url scheme used is `https://{account}.blob.core.windows.net`
    /// When enabled the url scheme used is `https://{account}.dfs.fabric.microsoft.com`
    ///
    /// Note: [`Self::with_endpoint`] will take precedence over this option
    pub fn with_use_fabric_endpoint(mut self, use_fabric_endpoint: bool) -> Self {
        self.use_fabric_endpoint = use_fabric_endpoint.into();
        self
    }

    /// Sets what protocol is allowed
    ///
    /// If `allow_http` is :
    /// * false (default):  Only HTTPS are allowed
    /// * true:  HTTP and HTTPS are allowed
    pub fn with_allow_http(mut self, allow_http: bool) -> Self {
        self.client_options = self.client_options.with_allow_http(allow_http);
        self
    }

    /// Sets an alternative authority host for OAuth based authorization
    ///
    /// Common hosts for azure clouds are defined in [authority_hosts](crate::azure::authority_hosts).
    ///
    /// Defaults to <https://login.microsoftonline.com>
    pub fn with_authority_host(mut self, authority_host: impl Into<String>) -> Self {
        self.authority_host = Some(authority_host.into());
        self
    }

    /// Set the retry configuration
    pub fn with_retry(mut self, retry_config: RetryConfig) -> Self {
        self.retry_config = retry_config;
        self
    }

    /// Set the proxy_url to be used by the underlying client
    pub fn with_proxy_url(mut self, proxy_url: impl Into<String>) -> Self {
        self.client_options = self.client_options.with_proxy_url(proxy_url);
        self
    }

    /// Set a trusted proxy CA certificate
    pub fn with_proxy_ca_certificate(mut self, proxy_ca_certificate: impl Into<String>) -> Self {
        self.client_options = self
            .client_options
            .with_proxy_ca_certificate(proxy_ca_certificate);
        self
    }

    /// Set a list of hosts to exclude from proxy connections
    pub fn with_proxy_excludes(mut self, proxy_excludes: impl Into<String>) -> Self {
        self.client_options = self.client_options.with_proxy_excludes(proxy_excludes);
        self
    }

    /// Sets the client options, overriding any already set
    pub fn with_client_options(mut self, options: ClientOptions) -> Self {
        self.client_options = options;
        self
    }

    /// Sets the endpoint for acquiring managed identity token
    pub fn with_msi_endpoint(mut self, msi_endpoint: impl Into<String>) -> Self {
        self.msi_endpoint = Some(msi_endpoint.into());
        self
    }

    /// Sets a file path for acquiring azure federated identity token in k8s
    ///
    /// requires `client_id` and `tenant_id` to be set
    pub fn with_federated_token_file(mut self, federated_token_file: impl Into<String>) -> Self {
        self.federated_token_file = Some(federated_token_file.into());
        self
    }

    /// Set if the Azure Cli should be used for acquiring access token
    ///
    /// <https://learn.microsoft.com/en-us/cli/azure/account?view=azure-cli-latest#az-account-get-access-token>
    pub fn with_use_azure_cli(mut self, use_azure_cli: bool) -> Self {
        self.use_azure_cli = use_azure_cli.into();
        self
    }

    /// Set the credential type to use for authentication.
    ///
    /// When multiple credential configurations are present (e.g. both workload identity
    /// and client secret), this forces the builder to use a specific credential type
    /// instead of relying on the default resolution order.
    ///
    /// Supported values: `auto`, `bearer_token`, `access_key`, `client_secret`,
    /// `workload_identity`, `sas_token`, `azure_cli`, `managed_identity`.
    pub fn with_credential_type(mut self, credential_type: impl Into<String>) -> Self {
        self.credential_type = Some(credential_type.into());
        self
    }

    /// If enabled, [`MicrosoftAzure`] will not fetch credentials and will not sign requests
    ///
    /// This can be useful when interacting with public containers
    pub fn with_skip_signature(mut self, skip_signature: bool) -> Self {
        self.skip_signature = skip_signature.into();
        self
    }

    /// If set to `true` will ignore any tags provided to put_opts
    pub fn with_disable_tagging(mut self, ignore: bool) -> Self {
        self.disable_tagging = ignore.into();
        self
    }

    /// Set the customer-provided encryption key (CPK) used to encrypt blob content.
    ///
    /// `key` must be a base64-encoded 256-bit AES key (the decoded value must be
    /// exactly 32 bytes). The same key must be supplied on every subsequent read,
    /// write, or copy of any blob created with it; if the key is lost or omitted
    /// the data is unrecoverable. CPK material is sent to Azure on every request,
    /// so the configured endpoint must use HTTPS.
    ///
    /// Only a subset of Blob storage operations support CPK
    /// (see the [Azure documentation][cpk-ops]). When CPK is enabled, `copy`
    /// switches from the asynchronous `Copy Blob` API to `Put Blob From URL`,
    /// which is synchronous and limits the source blob to 5,000 MiB.
    ///
    /// [cpk-ops]: https://learn.microsoft.com/en-us/azure/storage/blobs/encryption-customer-provided-keys#blob-storage-operations-supporting-customer-provided-keys
    pub fn with_encryption_key(mut self, key: impl Into<String>) -> Self {
        self.encryption_key = Some(key.into());
        self
    }

    /// The [`HttpConnector`] to use
    ///
    /// On non-WASM32 platforms uses [`reqwest`] by default, on WASM32 platforms must be provided
    pub fn with_http_connector<C: HttpConnector>(mut self, connector: C) -> Self {
        self.http_connector = Some(Arc::new(connector));
        self
    }

    fn resolve_credential_auto(
        &mut self,
        http: &Arc<dyn HttpConnector>,
    ) -> Result<AzureCredentialProvider> {
        if let (
            Some(fabric_token_service_url),
            Some(fabric_workload_host),
            Some(fabric_session_token),
            Some(fabric_cluster_identifier),
        ) = (
            &self.fabric_token_service_url,
            &self.fabric_workload_host,
            &self.fabric_session_token,
            &self.fabric_cluster_identifier,
        ) {
            let fabric_credential = FabricTokenOAuthProvider::new(
                fabric_token_service_url,
                fabric_workload_host,
                fabric_session_token,
                fabric_cluster_identifier,
                self.bearer_token.clone(),
            );
            Ok(Arc::new(TokenCredentialProvider::new(
                fabric_credential,
                http.connect(&self.client_options)?,
                self.retry_config.clone(),
            )) as _)
        } else if self.bearer_token.is_some() {
            self.resolve_bearer_token()
        } else if self.access_key.is_some() {
            self.resolve_access_key()
        } else if self.client_id.is_some()
            && self.tenant_id.is_some()
            && self.federated_token_file.is_some()
        {
            self.resolve_workload_identity(http)
        } else if self.client_id.is_some()
            && self.client_secret.is_some()
            && self.tenant_id.is_some()
        {
            self.resolve_client_secret(http)
        } else if self.sas_query_pairs.is_some() || self.sas_key.is_some() {
            self.resolve_sas_token()
        } else if self.use_azure_cli.get()? {
            Ok(Arc::new(AzureCliCredential::new()) as _)
        } else {
            self.resolve_managed_identity(http)
        }
    }

    fn resolve_bearer_token(&mut self) -> Result<AzureCredentialProvider> {
        let bearer_token = self
            .bearer_token
            .take()
            .ok_or(Error::MissingCredentialConfig {
                credential_type: "bearer_token".to_string(),
            })?;
        Ok(Arc::new(StaticCredentialProvider::new(
            AzureCredential::BearerToken(bearer_token),
        )))
    }

    fn resolve_access_key(&mut self) -> Result<AzureCredentialProvider> {
        let access_key = self
            .access_key
            .take()
            .ok_or(Error::MissingCredentialConfig {
                credential_type: "access_key".to_string(),
            })?;
        let key = AzureAccessKey::try_new(&access_key)?;
        Ok(Arc::new(StaticCredentialProvider::new(
            AzureCredential::AccessKey(key),
        )))
    }

    fn resolve_client_secret(
        &mut self,
        http: &Arc<dyn HttpConnector>,
    ) -> Result<AzureCredentialProvider> {
        let client_id = self
            .client_id
            .take()
            .ok_or(Error::MissingCredentialConfig {
                credential_type: "client_secret".to_string(),
            })?;
        let client_secret = self
            .client_secret
            .take()
            .ok_or(Error::MissingCredentialConfig {
                credential_type: "client_secret".to_string(),
            })?;
        let tenant_id = self
            .tenant_id
            .take()
            .ok_or(Error::MissingCredentialConfig {
                credential_type: "client_secret".to_string(),
            })?;
        let client_credential = ClientSecretOAuthProvider::new(
            client_id,
            client_secret,
            &tenant_id,
            self.authority_host.take(),
        );
        Ok(Arc::new(TokenCredentialProvider::new(
            client_credential,
            http.connect(&self.client_options)?,
            self.retry_config.clone(),
        )) as _)
    }

    fn resolve_workload_identity(
        &mut self,
        http: &Arc<dyn HttpConnector>,
    ) -> Result<AzureCredentialProvider> {
        let client_id = self
            .client_id
            .take()
            .ok_or(Error::MissingCredentialConfig {
                credential_type: "workload_identity".to_string(),
            })?;
        let tenant_id = self
            .tenant_id
            .take()
            .ok_or(Error::MissingCredentialConfig {
                credential_type: "workload_identity".to_string(),
            })?;
        let federated_token_file =
            self.federated_token_file
                .take()
                .ok_or(Error::MissingCredentialConfig {
                    credential_type: "workload_identity".to_string(),
                })?;
        let client_credential = WorkloadIdentityOAuthProvider::new(
            &client_id,
            federated_token_file,
            &tenant_id,
            self.authority_host.take(),
        );
        Ok(Arc::new(TokenCredentialProvider::new(
            client_credential,
            http.connect(&self.client_options)?,
            self.retry_config.clone(),
        )) as _)
    }

    fn resolve_sas_token(&mut self) -> Result<AzureCredentialProvider> {
        if let Some(query_pairs) = self.sas_query_pairs.take() {
            Ok(Arc::new(StaticCredentialProvider::new(
                AzureCredential::SASToken(query_pairs),
            )))
        } else if let Some(sas) = self.sas_key.take() {
            Ok(Arc::new(StaticCredentialProvider::new(
                AzureCredential::SASToken(split_sas(&sas)?),
            )))
        } else {
            Err(Error::MissingCredentialConfig {
                credential_type: "sas_token".to_string(),
            }
            .into())
        }
    }

    fn resolve_managed_identity(
        &mut self,
        http: &Arc<dyn HttpConnector>,
    ) -> Result<AzureCredentialProvider> {
        let msi_credential = ImdsManagedIdentityProvider::new(
            self.client_id.take(),
            self.object_id.take(),
            self.msi_resource_id.take(),
            self.msi_endpoint.take(),
        );
        Ok(Arc::new(TokenCredentialProvider::new(
            msi_credential,
            http.connect(&self.client_options.metadata_options())?,
            self.retry_config.clone(),
        )) as _)
    }

    /// Configure a connection to container with given name on Microsoft Azure Blob store.
    pub fn build(mut self) -> Result<MicrosoftAzure> {
        if let Some(url) = self.url.take() {
            self.parse_url(&url)?;
        }

        let container = self
            .container_name
            .take()
            .ok_or(Error::MissingContainerName {})?;

        let static_creds = |credential: AzureCredential| -> AzureCredentialProvider {
            Arc::new(StaticCredentialProvider::new(credential))
        };

        let http = http_connector(self.http_connector.take())?;

        let (is_emulator, storage_url, auth, account) = if self.use_emulator.get()? {
            let account_name = self
                .account_name
                .unwrap_or_else(|| EMULATOR_ACCOUNT.to_string());
            // Allow overriding defaults. Values taken from
            // from https://docs.rs/azure_storage/0.2.0/src/azure_storage/core/clients/storage_account_client.rs.html#129-141
            let url = url_from_env("AZURITE_BLOB_STORAGE_URL", "http://127.0.0.1:10000")?;
            let credential = if let Some(k) = self.access_key {
                AzureCredential::AccessKey(AzureAccessKey::try_new(&k)?)
            } else if let Some(bearer_token) = self.bearer_token {
                AzureCredential::BearerToken(bearer_token)
            } else if let Some(query_pairs) = self.sas_query_pairs {
                AzureCredential::SASToken(query_pairs)
            } else if let Some(sas) = self.sas_key {
                AzureCredential::SASToken(split_sas(&sas)?)
            } else {
                AzureCredential::AccessKey(AzureAccessKey::try_new(EMULATOR_ACCOUNT_KEY)?)
            };

            self.client_options = self.client_options.with_allow_http(true);
            (true, url, static_creds(credential), account_name)
        } else {
            let account_name = self.account_name.take().ok_or(Error::MissingAccount {})?;
            let account_url = match self.endpoint.take() {
                Some(account_url) => account_url,
                None => match self.use_fabric_endpoint.get()? {
                    true => {
                        format!("https://{}.blob.fabric.microsoft.com", account_name)
                    }
                    false => format!("https://{}.blob.core.windows.net", account_name),
                },
            };

            let url = Url::parse(&account_url).map_err(|source| {
                let url = account_url.clone();
                Error::UnableToParseUrl { url, source }
            })?;

            let credential = if let Some(credential) = self.credentials {
                credential
            } else {
                let credential_type = self.credential_type.as_deref().unwrap_or("auto");

                match credential_type {
                    "auto" => self.resolve_credential_auto(&http)?,
                    "bearer_token" => self.resolve_bearer_token()?,
                    "access_key" => self.resolve_access_key()?,
                    "client_secret" => self.resolve_client_secret(&http)?,
                    "workload_identity" => self.resolve_workload_identity(&http)?,
                    "sas_token" => self.resolve_sas_token()?,
                    "azure_cli" => Arc::new(AzureCliCredential::new()) as _,
                    "managed_identity" => self.resolve_managed_identity(&http)?,
                    other => {
                        return Err(Error::UnknownCredentialType {
                            credential_type: other.to_string(),
                        }
                        .into());
                    }
                }
            };
            (false, url, credential, account_name)
        };

        let encryption_headers =
            AzureEncryptionHeaders::try_new(self.crypto.as_deref(), self.encryption_key).map_err(
                |source| Error::InvalidEncryptionKey {
                    source: match source {
                        crate::Error::Generic { source, .. } => source,
                        other => Box::new(other),
                    },
                },
            )?;

        let config = AzureConfig {
            account,
            is_emulator,
            skip_signature: self.skip_signature.get()?,
            container,
            disable_tagging: self.disable_tagging.get()?,
            retry_config: self.retry_config,
            client_options: self.client_options,
            service: storage_url,
            credentials: auth,
            crypto: self.crypto,
            encryption_headers,
        };

        let http_client = http.connect(&config.client_options)?;
        let client = Arc::new(AzureClient::new(config, http_client));

        Ok(MicrosoftAzure { client })
    }
}

/// Parses the contents of the environment variable `env_name` as a URL
/// if present, otherwise falls back to default_url
fn url_from_env(env_name: &str, default_url: &str) -> Result<Url> {
    let url = match std::env::var(env_name) {
        Ok(env_value) => {
            Url::parse(&env_value).map_err(|source| Error::UnableToParseEmulatorUrl {
                env_name: env_name.into(),
                env_value,
                source,
            })?
        }
        Err(_) => Url::parse(default_url).expect("Failed to parse default URL"),
    };
    Ok(url)
}

/// Parse a SAS token string into the query pairs expected by [`AzureCredential::SASToken`].
pub fn split_sas(sas: &str) -> Result<Vec<(String, String)>> {
    let sas = percent_decode_str(sas)
        .decode_utf8()
        .map_err(|source| Error::DecodeSasKey { source })?;
    let kv_str_pairs = sas
        .trim_start_matches('?')
        .split('&')
        .filter(|s| !s.chars().all(char::is_whitespace));
    let mut pairs = Vec::new();
    for kv_pair_str in kv_str_pairs {
        let (k, v) = kv_pair_str
            .trim()
            .split_once('=')
            .ok_or(Error::MissingSasComponent {})?;
        pairs.push((k.into(), v.into()))
    }
    Ok(pairs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use base64::prelude::BASE64_STANDARD;
    use std::collections::HashMap;

    #[test]
    fn azure_blob_test_urls() {
        let mut builder = MicrosoftAzureBuilder::new();
        builder
            .parse_url("abfss://file_system@account.dfs.core.windows.net/")
            .unwrap();
        assert_eq!(builder.account_name, Some("account".to_string()));
        assert_eq!(builder.container_name, Some("file_system".to_string()));
        assert!(!builder.use_fabric_endpoint.get().unwrap());

        let mut builder = MicrosoftAzureBuilder::new();
        builder
            .parse_url("az://container@account.dfs.core.windows.net/path-part/file")
            .unwrap();
        assert_eq!(builder.account_name, Some("account".to_string()));
        assert_eq!(builder.container_name, Some("container".to_string()));
        assert!(!builder.use_fabric_endpoint.get().unwrap());

        let mut builder = MicrosoftAzureBuilder::new();
        builder
            .parse_url("az://container@account.blob.core.windows.net/path-part/file")
            .unwrap();
        assert_eq!(builder.account_name, Some("account".to_string()));
        assert_eq!(builder.container_name, Some("container".to_string()));
        assert!(!builder.use_fabric_endpoint.get().unwrap());

        let mut builder = MicrosoftAzureBuilder::new();
        builder
            .parse_url("az://container@account.dfs.fabric.microsoft.com/path-part/file")
            .unwrap();
        assert_eq!(builder.account_name, Some("account".to_string()));
        assert_eq!(builder.container_name, Some("container".to_string()));
        assert!(builder.use_fabric_endpoint.get().unwrap());

        let mut builder = MicrosoftAzureBuilder::new();
        builder
            .parse_url("az://container@account.blob.fabric.microsoft.com/path-part/file")
            .unwrap();
        assert_eq!(builder.account_name, Some("account".to_string()));
        assert_eq!(builder.container_name, Some("container".to_string()));
        assert!(builder.use_fabric_endpoint.get().unwrap());

        let mut builder = MicrosoftAzureBuilder::new();
        builder
            .parse_url("abfss://file_system@account.dfs.fabric.microsoft.com/")
            .unwrap();
        assert_eq!(builder.account_name, Some("account".to_string()));
        assert_eq!(builder.container_name, Some("file_system".to_string()));
        assert!(builder.use_fabric_endpoint.get().unwrap());

        let mut builder = MicrosoftAzureBuilder::new();
        builder.parse_url("abfs://container/path").unwrap();
        assert_eq!(builder.container_name, Some("container".to_string()));

        let mut builder = MicrosoftAzureBuilder::new();
        builder.parse_url("az://container").unwrap();
        assert_eq!(builder.container_name, Some("container".to_string()));

        let mut builder = MicrosoftAzureBuilder::new();
        builder.parse_url("az://container/path").unwrap();
        assert_eq!(builder.container_name, Some("container".to_string()));

        let mut builder = MicrosoftAzureBuilder::new();
        builder
            .parse_url("https://account.dfs.core.windows.net/")
            .unwrap();
        assert_eq!(builder.account_name, Some("account".to_string()));
        assert!(!builder.use_fabric_endpoint.get().unwrap());

        let mut builder =
            MicrosoftAzureBuilder::new().with_container_name("explicit_container_name");
        builder
            .parse_url("https://account.blob.core.windows.net/")
            .unwrap();
        assert_eq!(builder.account_name, Some("account".to_string()));
        assert_eq!(
            builder.container_name,
            Some("explicit_container_name".to_string())
        );
        assert!(!builder.use_fabric_endpoint.get().unwrap());

        let mut builder = MicrosoftAzureBuilder::new();
        builder
            .parse_url("https://account.blob.core.windows.net/container")
            .unwrap();
        assert_eq!(builder.account_name, Some("account".to_string()));
        assert_eq!(builder.container_name, Some("container".to_string()));
        assert!(!builder.use_fabric_endpoint.get().unwrap());

        let mut builder = MicrosoftAzureBuilder::new();
        builder
            .parse_url("https://account.dfs.fabric.microsoft.com/")
            .unwrap();
        assert_eq!(builder.account_name, Some("account".to_string()));
        assert_eq!(builder.container_name, None);
        assert!(builder.use_fabric_endpoint.get().unwrap());

        let mut builder = MicrosoftAzureBuilder::new();
        builder
            .parse_url("https://account.dfs.fabric.microsoft.com/container")
            .unwrap();
        assert_eq!(builder.account_name, Some("account".to_string()));
        assert_eq!(builder.container_name.as_deref(), Some("container"));
        assert!(builder.use_fabric_endpoint.get().unwrap());

        let mut builder = MicrosoftAzureBuilder::new();
        builder
            .parse_url("https://onelake.dfs.fabric.microsoft.com/c047b3e3-4e89-407a-98d7-cf9949ae92a3/9f1a2b3c-4d5e-6f70-8a9b-c0d1e2f3a456.lakehouse/Files/tables/sales/data.parquet")
            .unwrap();
        assert_eq!(builder.account_name, Some("onelake".to_string()));
        assert_eq!(
            builder.container_name.as_deref(),
            Some("c047b3e3-4e89-407a-98d7-cf9949ae92a3")
        );
        assert!(builder.use_fabric_endpoint.get().unwrap());

        let mut builder = MicrosoftAzureBuilder::new();
        builder
            .parse_url("https://account.blob.fabric.microsoft.com/")
            .unwrap();
        assert_eq!(builder.account_name, Some("account".to_string()));
        assert_eq!(builder.container_name, None);
        assert!(builder.use_fabric_endpoint.get().unwrap());

        let mut builder = MicrosoftAzureBuilder::new();
        builder
            .parse_url("https://account.blob.fabric.microsoft.com/container")
            .unwrap();
        assert_eq!(builder.account_name, Some("account".to_string()));
        assert_eq!(builder.container_name.as_deref(), Some("container"));
        assert!(builder.use_fabric_endpoint.get().unwrap());

        let err_cases = [
            "mailto://account.blob.core.windows.net/",
            "az://blob.mydomain/",
            "abfs://container.foo/path",
            "abfss://file_system@account.foo.dfs.core.windows.net/",
            "abfss://file_system.bar@account.dfs.core.windows.net/",
            "https://blob.mydomain/",
            "https://blob.foo.dfs.core.windows.net/",
        ];
        let mut builder = MicrosoftAzureBuilder::new();
        for case in err_cases {
            builder.parse_url(case).unwrap_err();
        }
    }

    #[test]
    fn azure_test_workspace_private_link() {
        let test_cases: Vec<(&str, &str, Option<&str>)> = vec![
            (
                "https://Ab000000000000000000000000000000.zAb.dfs.fabric.microsoft.com/",
                "ab000000000000000000000000000000.zab",
                None,
            ),
            (
                "https://ab000000000000000000000000000000.zab.dfs.fabric.microsoft.com/",
                "ab000000000000000000000000000000.zab",
                None,
            ),
            (
                "https://c047b3e34e89407a98d7cf9949ae92a3.zc0.blob.fabric.microsoft.com/c047b3e3-4e89-407a-98d7-cf9949ae92a3/9f1a2b3c-4d5e-6f70-8a9b-c0d1e2f3a456/file",
                "c047b3e34e89407a98d7cf9949ae92a3.zc0",
                Some("c047b3e3-4e89-407a-98d7-cf9949ae92a3"),
            ),
            (
                "https://c047b3e34e89407a98d7cf9949ae92a3.zc0.dfs.fabric.microsoft.com/c047b3e3-4e89-407a-98d7-cf9949ae92a3/9f1a2b3c-4d5e-6f70-8a9b-c0d1e2f3a456/file",
                "c047b3e34e89407a98d7cf9949ae92a3.zc0",
                Some("c047b3e3-4e89-407a-98d7-cf9949ae92a3"),
            ),
            (
                "https://c047b3e34e89407a98d7cf9949ae92a3.zc0.onelake.fabric.microsoft.com/c047b3e3-4e89-407a-98d7-cf9949ae92a3/9f1a2b3c-4d5e-6f70-8a9b-c0d1e2f3a456/file",
                "c047b3e34e89407a98d7cf9949ae92a3.zc0",
                Some("c047b3e3-4e89-407a-98d7-cf9949ae92a3"),
            ),
            (
                "https://c047b3e34e89407a98d7cf9949ae92a3.zc0.w.api.fabric.microsoft.com/c047b3e3-4e89-407a-98d7-cf9949ae92a3/9f1a2b3c-4d5e-6f70-8a9b-c0d1e2f3a456/file",
                "c047b3e34e89407a98d7cf9949ae92a3.zc0",
                Some("c047b3e3-4e89-407a-98d7-cf9949ae92a3"),
            ),
            (
                "https://c047b3e34e89407a98d7cf9949ae92a3.zc0.c.api.fabric.microsoft.com/c047b3e3-4e89-407a-98d7-cf9949ae92a3/9f1a2b3c-4d5e-6f70-8a9b-c0d1e2f3a456/file",
                "c047b3e34e89407a98d7cf9949ae92a3.zc0",
                Some("c047b3e3-4e89-407a-98d7-cf9949ae92a3"),
            ),
            (
                "abfss://c047b3e34e89407a98d7cf9949ae92a3@c047b3e34e89407a98d7cf9949ae92a3.zc0.dfs.fabric.microsoft.com/9f1a2b3c-4d5e-6f70-8a9b-c0d1e2f3a456/file",
                "c047b3e34e89407a98d7cf9949ae92a3.zc0",
                Some("c047b3e34e89407a98d7cf9949ae92a3"),
            ),
            (
                "abfss://c047b3e34e89407a98d7cf9949ae92a3@c047b3e34e89407a98d7cf9949ae92a3.zc0.blob.fabric.microsoft.com/9f1a2b3c-4d5e-6f70-8a9b-c0d1e2f3a456/file",
                "c047b3e34e89407a98d7cf9949ae92a3.zc0",
                Some("c047b3e34e89407a98d7cf9949ae92a3"),
            ),
        ];

        for (url, expected_account, expected_container) in &test_cases {
            let mut builder = MicrosoftAzureBuilder::new();
            builder.parse_url(url).unwrap();

            assert_eq!(
                builder.account_name.as_deref(),
                Some(*expected_account),
                "account mismatch for URL: {url}"
            );
            assert_eq!(
                builder.container_name.as_deref(),
                *expected_container,
                "container mismatch for URL: {url}"
            );
            assert!(
                builder.use_fabric_endpoint.get().unwrap(),
                "use_fabric_endpoint not set for URL: {url}"
            );
        }
    }

    #[test]
    fn azure_encryption_key_roundtrip() {
        let key = BASE64_STANDARD.encode([7_u8; 32]);
        let builder = MicrosoftAzureBuilder::new().with_encryption_key(&key);

        assert_eq!(
            builder
                .get_config_value(&AzureConfigKey::EncryptionKey)
                .as_deref(),
            Some(key.as_str())
        );
    }

    #[test]
    fn azure_encryption_key_rejects_malformed_base64() {
        let err = MicrosoftAzureBuilder::new()
            .with_account("account")
            .with_container_name("container")
            .with_access_key(EMULATOR_ACCOUNT_KEY)
            .with_encryption_key("not-base64!!!")
            .build()
            .unwrap_err();

        let msg = err.to_string();
        assert!(msg.contains("Invalid encryption key") || msg.contains("Invalid byte"));
    }

    #[test]
    fn azure_encryption_key_rejects_short_decoded_key() {
        let err = MicrosoftAzureBuilder::new()
            .with_account("account")
            .with_container_name("container")
            .with_access_key(EMULATOR_ACCOUNT_KEY)
            .with_encryption_key(BASE64_STANDARD.encode([7_u8; 31]))
            .build()
            .unwrap_err();

        assert!(err.to_string().contains("must decode to 32 bytes, got 31"));
    }

    #[test]
    fn azure_encryption_key_rejects_long_decoded_key() {
        let err = MicrosoftAzureBuilder::new()
            .with_account("account")
            .with_container_name("container")
            .with_access_key(EMULATOR_ACCOUNT_KEY)
            .with_encryption_key(BASE64_STANDARD.encode([7_u8; 33]))
            .build()
            .unwrap_err();

        assert!(err.to_string().contains("must decode to 32 bytes, got 33"));
    }

    #[test]
    fn azure_test_config_from_map() {
        let azure_client_id = "object_store:fake_access_key_id";
        let azure_storage_account_name = "object_store:fake_secret_key";
        let azure_storage_token = "object_store:fake_default_region";
        let options = HashMap::from([
            ("azure_client_id", azure_client_id),
            ("azure_storage_account_name", azure_storage_account_name),
            ("azure_storage_token", azure_storage_token),
        ]);

        let builder = options
            .into_iter()
            .fold(MicrosoftAzureBuilder::new(), |builder, (key, value)| {
                builder.with_config(key.parse().unwrap(), value)
            });
        assert_eq!(builder.client_id.unwrap(), azure_client_id);
        assert_eq!(builder.account_name.unwrap(), azure_storage_account_name);
        assert_eq!(builder.bearer_token.unwrap(), azure_storage_token);
    }

    #[test]
    fn azure_test_credential_type_config() {
        let builder = MicrosoftAzureBuilder::new()
            .with_config(AzureConfigKey::CredentialType, "client_secret");
        assert_eq!(builder.credential_type, Some("client_secret".to_string()));
        assert_eq!(
            builder
                .get_config_value(&AzureConfigKey::CredentialType)
                .unwrap(),
            "client_secret"
        );

        let builder =
            MicrosoftAzureBuilder::new().with_config("credential_type".parse().unwrap(), "auto");
        assert_eq!(builder.credential_type, Some("auto".to_string()));
    }

    #[test]
    fn azure_test_credential_type_client_secret() {
        let builder = MicrosoftAzureBuilder::new()
            .with_account("account")
            .with_container_name("container")
            .with_client_id("client_id")
            .with_client_secret("client_secret")
            .with_tenant_id("tenant_id")
            .with_federated_token_file("/tmp/token")
            .with_credential_type("client_secret");
        // Should succeed — client_secret is forced even though workload_identity fields are present
        let result = builder.build();
        // Build will fail because it can't actually connect, but it should not fail with
        // a MissingCredentialConfig error — it should get past credential resolution
        assert!(
            result.is_ok(),
            "Expected build to succeed with credential_type=client_secret, got: {:?}",
            result.err()
        );
    }

    #[test]
    fn azure_test_credential_type_unknown() {
        let builder = MicrosoftAzureBuilder::new()
            .with_account("account")
            .with_container_name("container")
            .with_credential_type("invalid_type");
        let result = builder.build();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Unknown credential type"),
            "Expected unknown credential type error, got: {err}"
        );
    }

    #[test]
    fn azure_test_credential_type_missing_config() {
        let builder = MicrosoftAzureBuilder::new()
            .with_account("account")
            .with_container_name("container")
            .with_credential_type("client_secret");
        let result = builder.build();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("required configuration is missing"),
            "Expected missing config error, got: {err}"
        );
    }

    #[test]
    fn azure_test_split_sas() {
        let raw_sas = "?sv=2021-10-04&st=2023-01-04T17%3A48%3A57Z&se=2023-01-04T18%3A15%3A00Z&sr=c&sp=rcwl&sig=C7%2BZeEOWbrxPA3R0Cw%2Fw1EZz0%2B4KBvQexeKZKe%2BB6h0%3D";
        let expected = vec![
            ("sv".to_string(), "2021-10-04".to_string()),
            ("st".to_string(), "2023-01-04T17:48:57Z".to_string()),
            ("se".to_string(), "2023-01-04T18:15:00Z".to_string()),
            ("sr".to_string(), "c".to_string()),
            ("sp".to_string(), "rcwl".to_string()),
            (
                "sig".to_string(),
                "C7+ZeEOWbrxPA3R0Cw/w1EZz0+4KBvQexeKZKe+B6h0=".to_string(),
            ),
        ];
        let pairs = split_sas(raw_sas).unwrap();
        assert_eq!(expected, pairs);
    }

    #[test]
    fn azure_test_split_sas_trims_leading_question_mark_and_skips_empties() {
        let pairs = split_sas("?&sv=2021-10-04& &sp=r").unwrap();
        assert_eq!(
            pairs,
            vec![
                ("sv".to_string(), "2021-10-04".to_string()),
                ("sp".to_string(), "r".to_string()),
            ],
        );
    }

    #[test]
    fn azure_test_split_sas_rejects_missing_equals() {
        let err = split_sas("sv=2021-10-04&bogus").unwrap_err();
        assert!(
            err.to_string().contains("Missing component"),
            "unexpected error: {err}",
        );
    }

    #[test]
    fn azure_test_client_opts() {
        let key = "AZURE_PROXY_URL";
        if let Ok(config_key) = key.to_ascii_lowercase().parse() {
            assert_eq!(
                AzureConfigKey::Client(ClientConfigKey::ProxyUrl),
                config_key
            );
        } else {
            panic!("{key} not propagated as ClientConfigKey");
        }
    }
}
