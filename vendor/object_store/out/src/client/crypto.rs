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

use crate::Result;

/// Algorithm for computing digests
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
#[non_exhaustive]
pub enum DigestAlgorithm {
    /// SHA-256
    Sha256,
}

/// Algorithm for signing payloads
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
#[non_exhaustive]
pub enum SigningAlgorithm {
    /// RSASSA-PKCS1-v1_5 using SHA-256
    RS256,
}

/// Provides cryptographic primitives
pub trait CryptoProvider: std::fmt::Debug + Send + Sync {
    /// Compute a digest
    fn digest(&self, algorithm: DigestAlgorithm) -> Result<Box<dyn DigestContext>>;

    /// Compute an HMAC with the provided `secret`
    fn hmac(&self, algorithm: DigestAlgorithm, secret: &[u8]) -> Result<Box<dyn HmacContext>>;

    /// Sign a payload with the provided PEM-encoded secret
    fn sign(&self, algorithm: SigningAlgorithm, pem: &[u8]) -> Result<Box<dyn Signer>>;
}

/// Incrementally compute a digest, see [`CryptoProvider::digest`]
pub trait DigestContext: Send {
    /// Updates the digest with all the data in data.
    ///
    /// It is implementation-defined behaviour to call this after calling [`Self::finish`]
    fn update(&mut self, data: &[u8]);

    /// Finalizes the digest calculation and returns the digest value.
    ///
    /// It is implementation-defined behaviour to call this after calling [`Self::finish`]
    fn finish(&mut self) -> Result<&[u8]>;
}

/// Incrementally compute a HMAC, see [`CryptoProvider::hmac`]
pub trait HmacContext: Send {
    /// Updates the HMAC with all the data in data.
    ///
    /// It is implementation-defined behaviour to call this after calling [`Self::finish`]
    fn update(&mut self, data: &[u8]);

    /// Finalizes the HMAC calculation and returns the HMAC value.
    ///
    /// It is implementation-defined behaviour to call this after calling [`Self::finish`]
    fn finish(&mut self) -> Result<&[u8]>;
}

/// Sign a payload, see [`CryptoProvider::sign`]
pub trait Signer: Send + Sync {
    /// Sign the provided payload
    fn sign(&self, string_to_sign: &[u8]) -> Result<Vec<u8>>;
}

/// Attempts to find a [`CryptoProvider`]
///
/// If `custom` is `Some(v)` returns `v` otherwise returns the compile-time default
///
/// If both `ring` and `aws-lc-rs` are enabled, the `aws-lc-rs` provider is used.
pub(crate) fn crypto_provider(custom: Option<&dyn CryptoProvider>) -> Result<&dyn CryptoProvider> {
    if let Some(x) = custom {
        return Ok(x);
    }

    #[cfg(feature = "aws-lc-rs")]
    {
        Ok(&aws_lc_rs::PROVIDER)
    }

    #[cfg(all(feature = "ring", not(feature = "aws-lc-rs")))]
    {
        Ok(&ring::PROVIDER)
    }

    #[cfg(not(any(feature = "ring", feature = "aws-lc-rs")))]
    {
        Err(crate::Error::NotSupported {
            source: "Must enable aws-lc-rs, ring, or specify custom CryptoProvider"
                .to_string()
                .into(),
        })
    }
}

#[cfg(all(feature = "ring", not(feature = "aws-lc-rs")))]
pub(crate) mod ring {
    use super::*;
    use ::ring::{digest, hmac, rand, signature};
    use thiserror::Error;

    #[derive(Debug, Error)]
    pub(crate) enum RingError {
        #[error("No RSA key found in pem file")]
        MissingKey,

        #[error("Invalid RSA key: {}", source)]
        InvalidKey {
            #[from]
            source: ::ring::error::KeyRejected,
        },

        #[error("Error reading pem file: {}", source)]
        ReadPem {
            source: rustls_pki_types::pem::Error,
        },

        #[error("Error signing: {}", source)]
        Sign { source: ::ring::error::Unspecified },
    }

    impl From<RingError> for crate::Error {
        fn from(value: RingError) -> Self {
            Self::Generic {
                store: "RingCryptoProvider",
                source: Box::new(value),
            }
        }
    }

    pub(crate) const PROVIDER: RingCryptoProvider = RingCryptoProvider { _private: () };

    #[derive(Debug, Default)]
    pub(crate) struct RingCryptoProvider {
        _private: (),
    }

    impl CryptoProvider for RingCryptoProvider {
        fn digest(&self, algorithm: DigestAlgorithm) -> Result<Box<dyn DigestContext>> {
            let algorithm = match algorithm {
                DigestAlgorithm::Sha256 => &digest::SHA256,
            };
            let ctx = digest::Context::new(algorithm);
            Ok(Box::new(RingDigestContext {
                ctx: Some(ctx),
                out: None,
            }))
        }

        fn hmac(&self, algorithm: DigestAlgorithm, secret: &[u8]) -> Result<Box<dyn HmacContext>> {
            let algorithm = match algorithm {
                DigestAlgorithm::Sha256 => hmac::HMAC_SHA256,
            };
            let ctx = hmac::Context::with_key(&hmac::Key::new(algorithm, secret));
            Ok(Box::new(RingHmacContext {
                ctx: Some(ctx),
                out: None,
            }))
        }

        fn sign(&self, algorithm: SigningAlgorithm, pem: &[u8]) -> Result<Box<dyn Signer>> {
            match algorithm {
                SigningAlgorithm::RS256 => Ok(Box::new(RsaKeyPair::from_pem(pem)?)),
            }
        }
    }

    struct RingDigestContext {
        ctx: Option<digest::Context>,
        out: Option<digest::Digest>,
    }

    impl DigestContext for RingDigestContext {
        fn update(&mut self, data: &[u8]) {
            self.ctx.as_mut().unwrap().update(data);
        }

        fn finish(&mut self) -> Result<&[u8]> {
            let digest = self.ctx.take().unwrap().finish();
            Ok(digest::Digest::as_ref(self.out.insert(digest)))
        }
    }

    struct RingHmacContext {
        ctx: Option<hmac::Context>,
        out: Option<hmac::Tag>,
    }

    impl HmacContext for RingHmacContext {
        fn update(&mut self, data: &[u8]) {
            self.ctx.as_mut().unwrap().update(data);
        }

        fn finish(&mut self) -> Result<&[u8]> {
            let tag = self.ctx.take().unwrap().sign();
            Ok(hmac::Tag::as_ref(self.out.insert(tag)))
        }
    }

    /// A private RSA key for a service account
    #[derive(Debug)]
    pub(crate) struct RsaKeyPair(signature::RsaKeyPair);

    impl RsaKeyPair {
        /// Parses a pem-encoded RSA key
        pub(crate) fn from_pem(encoded: &[u8]) -> Result<Self, RingError> {
            use rustls_pki_types::PrivateKeyDer;
            use rustls_pki_types::pem::PemObject;

            match PrivateKeyDer::from_pem_slice(encoded) {
                Ok(PrivateKeyDer::Pkcs8(key)) => Self::from_pkcs8(key.secret_pkcs8_der()),
                Ok(PrivateKeyDer::Pkcs1(key)) => Self::from_der(key.secret_pkcs1_der()),
                Ok(_) => Err(RingError::MissingKey),
                Err(source) => Err(RingError::ReadPem { source }),
            }
        }

        /// Parses an unencrypted PKCS#8-encoded RSA private key.
        pub(crate) fn from_pkcs8(key: &[u8]) -> Result<Self, RingError> {
            Ok(Self(signature::RsaKeyPair::from_pkcs8(key)?))
        }

        /// Parses an unencrypted PKCS#8-encoded RSA private key.
        pub(crate) fn from_der(key: &[u8]) -> Result<Self, RingError> {
            Ok(Self(signature::RsaKeyPair::from_der(key)?))
        }
    }

    impl Signer for RsaKeyPair {
        fn sign(&self, string_to_sign: &[u8]) -> Result<Vec<u8>> {
            let mut signature = vec![0; self.0.public().modulus_len()];
            self.0
                .sign(
                    &signature::RSA_PKCS1_SHA256,
                    &rand::SystemRandom::new(),
                    string_to_sign,
                    &mut signature,
                )
                .map_err(|source| RingError::Sign { source })?;

            Ok(signature)
        }
    }
}

#[cfg(feature = "aws-lc-rs")]
pub(crate) mod aws_lc_rs {
    use super::*;
    use ::aws_lc_rs::{digest, hmac, rand, signature};
    use thiserror::Error;

    #[derive(Debug, Error)]
    pub(crate) enum AwsLcError {
        #[error("No RSA key found in pem file")]
        MissingKey,

        #[error("Invalid RSA key: {}", source)]
        InvalidKey {
            #[from]
            source: ::aws_lc_rs::error::KeyRejected,
        },

        #[error("Error reading pem file: {}", source)]
        ReadPem {
            source: rustls_pki_types::pem::Error,
        },

        #[error("Error signing: {}", source)]
        Sign {
            source: ::aws_lc_rs::error::Unspecified,
        },
    }

    impl From<AwsLcError> for crate::Error {
        fn from(value: AwsLcError) -> Self {
            Self::Generic {
                store: "AwsLcCryptoProvider",
                source: Box::new(value),
            }
        }
    }

    pub(crate) const PROVIDER: AwsLcCryptoProvider = AwsLcCryptoProvider { _private: () };

    #[derive(Debug, Default)]
    pub(crate) struct AwsLcCryptoProvider {
        _private: (),
    }

    impl CryptoProvider for AwsLcCryptoProvider {
        fn digest(&self, algorithm: DigestAlgorithm) -> Result<Box<dyn DigestContext>> {
            let algorithm = match algorithm {
                DigestAlgorithm::Sha256 => &digest::SHA256,
            };
            let ctx = digest::Context::new(algorithm);
            Ok(Box::new(AwsLcDigestContext {
                ctx: Some(ctx),
                out: None,
            }))
        }

        fn hmac(&self, algorithm: DigestAlgorithm, secret: &[u8]) -> Result<Box<dyn HmacContext>> {
            let algorithm = match algorithm {
                DigestAlgorithm::Sha256 => hmac::HMAC_SHA256,
            };
            let ctx = hmac::Context::with_key(&hmac::Key::new(algorithm, secret));
            Ok(Box::new(AwsLcHmacContext {
                ctx: Some(ctx),
                out: None,
            }))
        }

        fn sign(&self, algorithm: SigningAlgorithm, pem: &[u8]) -> Result<Box<dyn Signer>> {
            match algorithm {
                SigningAlgorithm::RS256 => Ok(Box::new(RsaKeyPair::from_pem(pem)?)),
            }
        }
    }

    struct AwsLcDigestContext {
        ctx: Option<digest::Context>,
        out: Option<digest::Digest>,
    }

    impl DigestContext for AwsLcDigestContext {
        fn update(&mut self, data: &[u8]) {
            self.ctx.as_mut().unwrap().update(data);
        }

        fn finish(&mut self) -> Result<&[u8]> {
            let digest = self.ctx.take().unwrap().finish();
            Ok(digest::Digest::as_ref(self.out.insert(digest)))
        }
    }

    struct AwsLcHmacContext {
        ctx: Option<hmac::Context>,
        out: Option<hmac::Tag>,
    }

    impl HmacContext for AwsLcHmacContext {
        fn update(&mut self, data: &[u8]) {
            self.ctx.as_mut().unwrap().update(data);
        }

        fn finish(&mut self) -> Result<&[u8]> {
            let tag = self.ctx.take().unwrap().sign();
            Ok(hmac::Tag::as_ref(self.out.insert(tag)))
        }
    }

    /// A private RSA key for a service account
    #[derive(Debug)]
    pub(crate) struct RsaKeyPair(signature::RsaKeyPair);

    impl RsaKeyPair {
        /// Parses a pem-encoded RSA key
        pub(crate) fn from_pem(encoded: &[u8]) -> Result<Self, AwsLcError> {
            use rustls_pki_types::PrivateKeyDer;
            use rustls_pki_types::pem::PemObject;

            match PrivateKeyDer::from_pem_slice(encoded) {
                Ok(PrivateKeyDer::Pkcs8(key)) => Self::from_pkcs8(key.secret_pkcs8_der()),
                Ok(PrivateKeyDer::Pkcs1(key)) => Self::from_der(key.secret_pkcs1_der()),
                Ok(_) => Err(AwsLcError::MissingKey),
                Err(source) => Err(AwsLcError::ReadPem { source }),
            }
        }

        /// Parses an unencrypted PKCS#8-encoded RSA private key.
        pub(crate) fn from_pkcs8(key: &[u8]) -> Result<Self, AwsLcError> {
            Ok(Self(signature::RsaKeyPair::from_pkcs8(key)?))
        }

        /// Parses an unencrypted PKCS#8-encoded RSA private key.
        pub(crate) fn from_der(key: &[u8]) -> Result<Self, AwsLcError> {
            Ok(Self(signature::RsaKeyPair::from_der(key)?))
        }
    }

    impl Signer for RsaKeyPair {
        fn sign(&self, string_to_sign: &[u8]) -> Result<Vec<u8>> {
            let mut signature = vec![0; self.0.public_modulus_len()];
            self.0
                .sign(
                    &signature::RSA_PKCS1_SHA256,
                    &rand::SystemRandom::new(),
                    string_to_sign,
                    &mut signature,
                )
                .map_err(|source| AwsLcError::Sign { source })?;

            Ok(signature)
        }
    }
}
