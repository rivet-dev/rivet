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

//! Customizable DNS resolution for remote object stores

use std::fmt::Debug;
use std::future::Future;
use std::net::IpAddr;
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::pin::Pin;

/// Error returned by a [`DnsResolver`]
pub type DnsError = Box<dyn std::error::Error + Send + Sync>;

/// Future returned by [`DnsResolver::resolve`]
// NOTE: the use cases requiring SocketAddr over IpAddr (i.e., resolver-supplied ports
// and IPv6 scope IDs) do not apply to object_store, where the port is always
// determined by the endpoint URL/scheme and endpoints are never link-local.
pub type DnsFuture = Pin<Box<dyn Future<Output = Result<Vec<IpAddr>, DnsError>> + Send>>;

/// A custom DNS resolver used when establishing connections to remote object stores
///
/// This can be used to implement custom resolution logic such as caching,
/// address shuffling, or split-horizon DNS, independent of the underlying
/// HTTP transport.
///
/// Configure via [`ClientOptions::with_dns_resolver`]. The built-in
/// reqwest-based transport honors this automatically; custom
/// [`HttpConnector`] implementations should retrieve it via
/// [`ClientOptions::dns_resolver`] and apply it themselves.
///
/// [`ClientOptions::with_dns_resolver`]: crate::ClientOptions::with_dns_resolver
/// [`ClientOptions::dns_resolver`]: crate::ClientOptions::dns_resolver
/// [`HttpConnector`]: crate::client::HttpConnector
// The `UnwindSafe` bounds preserve the auto traits of `ClientOptions`, which
// stores an `Arc<dyn DnsResolver>`
pub trait DnsResolver: Debug + Send + Sync + UnwindSafe + RefUnwindSafe {
    /// Resolve `host` to one or more IP addresses
    ///
    /// The returned addresses are tried in order until a connection succeeds,
    /// so implementations are responsible for any ordering they require, e.g.
    /// shuffling or interleaving of address families. The port is determined
    /// by the transport from the URL, not by the resolver.
    fn resolve(&self, host: &str) -> DnsFuture;
}

#[cfg(feature = "reqwest")]
mod reqwest_impl {
    use super::{DnsError, DnsFuture, DnsResolver};
    use rand::prelude::SliceRandom;
    use std::net::{SocketAddr, ToSocketAddrs};
    use std::sync::Arc;
    use tokio::task::JoinSet;

    /// Adapts a [`DnsResolver`] to [`reqwest::dns::Resolve`]
    ///
    /// This is deliberately private: it is the only place where [`reqwest`]'s
    /// resolver API appears, keeping it out of this crate's public interface.
    pub(crate) struct ReqwestResolver(pub(crate) Arc<dyn DnsResolver>);

    impl reqwest::dns::Resolve for ReqwestResolver {
        fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
            let resolver = Arc::clone(&self.0);
            let host = name.as_str().to_string();
            Box::pin(async move {
                let ips = resolver.resolve(&host).await?;
                // Port 0 is a placeholder: reqwest documents that it is replaced
                // by the port from the URL or the scheme's conventional port
                let addrs: reqwest::dns::Addrs =
                    Box::new(ips.into_iter().map(|ip| SocketAddr::new(ip, 0)));
                Ok(addrs)
            })
        }
    }

    /// The built-in shuffling [`DnsResolver`], randomizing the order of the returned
    /// addresses to spread load across servers, see [`ClientConfigKey::RandomizeAddresses`]
    ///
    /// [`ClientConfigKey::RandomizeAddresses`]: crate::ClientConfigKey::RandomizeAddresses
    #[derive(Debug)]
    pub(crate) struct ShuffleResolver;

    impl DnsResolver for ShuffleResolver {
        fn resolve(&self, host: &str) -> DnsFuture {
            let host = host.to_string();
            Box::pin(async move {
                // use `JoinSet` to propagate cancellation to tasks that haven't started running yet.
                let mut tasks = JoinSet::new();
                tasks.spawn_blocking(move || {
                    let it = (host.as_str(), 0).to_socket_addrs()?;
                    let mut addrs = it.map(|addr| addr.ip()).collect::<Vec<_>>();
                    addrs.shuffle(&mut rand::rng());
                    Ok(addrs)
                });
                tasks
                    .join_next()
                    .await
                    .expect("spawned one task")
                    .map_err(|err| Box::new(err) as DnsError)?
            })
        }
    }
}

#[cfg(feature = "reqwest")]
pub(crate) use reqwest_impl::{ReqwestResolver, ShuffleResolver};

#[cfg(all(test, feature = "reqwest"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn shuffle_resolver_resolves_localhost() {
        let ips = ShuffleResolver.resolve("localhost").await.unwrap();
        assert!(!ips.is_empty());
        assert!(ips.iter().all(|ip| ip.is_loopback()));
    }

    #[derive(Debug)]
    struct FailingResolver;

    impl DnsResolver for FailingResolver {
        fn resolve(&self, _host: &str) -> DnsFuture {
            Box::pin(async { Err("boom".into()) })
        }
    }

    #[tokio::test]
    async fn adapter_propagates_errors() {
        use reqwest::dns::Resolve;
        use std::sync::Arc;
        let adapter = ReqwestResolver(Arc::new(FailingResolver));
        let err = match adapter.resolve("localhost".parse().unwrap()).await {
            Ok(_) => panic!("expected resolution to fail"),
            Err(e) => e,
        };
        assert!(err.to_string().contains("boom"));
    }
}
