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

use crate::config::Parse;

use itertools::Itertools;

/// Configure how to provide [`CopyMode::Create`] for [`AmazonS3`].
///
/// [`CopyMode::Create`]: crate::CopyMode::Create
/// [`AmazonS3`]: super::AmazonS3
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum S3CopyIfNotExists {
    /// Some S3-compatible stores, such as Cloudflare R2, support copy if not exists
    /// semantics through custom headers.
    ///
    /// If set, [`CopyMode::Create`] will perform a normal copy operation
    /// with the provided header pair, and expect the store to fail with `412 Precondition Failed`
    /// if the destination file already exists.
    ///
    /// Encoded as `header:<HEADER_NAME>:<HEADER_VALUE>` ignoring whitespace
    ///
    /// For example `header: cf-copy-destination-if-none-match: *`, would set
    /// the header `cf-copy-destination-if-none-match` to `*`
    ///
    /// [`CopyMode::Create`]: crate::CopyMode::Create
    Header(String, String),
    /// The same as [`S3CopyIfNotExists::Header`] but allows custom status code checking, for object stores that return values
    /// other than 412.
    ///
    /// Encoded as `header-with-status:<HEADER_NAME>:<HEADER_VALUE>:<STATUS>` ignoring whitespace
    HeaderWithStatus(String, String, http::StatusCode),
    /// Native Amazon S3 supports copy if not exists through a multipart upload
    /// where the upload copies an existing object and is completed only if the
    /// new object does not already exist.
    ///
    /// WARNING: When using this mode, `copy_if_not_exists` does not copy tags
    /// or attributes from the source object.
    ///
    /// WARNING: When using this mode, `copy_if_not_exists` makes only a best
    /// effort attempt to clean up the multipart upload if the copy operation
    /// fails. Consider using a lifecycle rule to automatically clean up
    /// abandoned multipart uploads. See [the module
    /// docs](super#multipart-uploads) for details.
    ///
    /// Encoded as `multipart` ignoring whitespace.
    Multipart,
}

impl std::fmt::Display for S3CopyIfNotExists {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Header(k, v) => write!(f, "header: {k}: {v}"),
            Self::HeaderWithStatus(k, v, code) => {
                write!(f, "header-with-status: {k}: {v}: {}", code.as_u16())
            }
            Self::Multipart => f.write_str("multipart"),
        }
    }
}

impl S3CopyIfNotExists {
    fn from_str(s: &str) -> Option<Self> {
        if s.trim() == "multipart" {
            return Some(Self::Multipart);
        };

        let (variant, value) = s.split_once(':')?;
        match variant.trim() {
            "header" => {
                let (k, v) = value.split_once(':')?;
                Some(Self::Header(k.trim().to_string(), v.trim().to_string()))
            }
            "header-with-status" => {
                let (k, v, status) = value.split(':').collect_tuple()?;

                let code = status.trim().parse().ok()?;

                Some(Self::HeaderWithStatus(
                    k.trim().to_string(),
                    v.trim().to_string(),
                    code,
                ))
            }
            _ => None,
        }
    }
}

impl Parse for S3CopyIfNotExists {
    fn parse(v: &str) -> crate::Result<Self> {
        Self::from_str(v).ok_or_else(|| crate::Error::Generic {
            store: "Config",
            source: format!("Failed to parse \"{v}\" as S3CopyIfNotExists").into(),
        })
    }
}

/// Configure how to provide conditional put support for [`AmazonS3`].
///
/// [`AmazonS3`]: super::AmazonS3
#[derive(Debug, Clone, Eq, PartialEq, Default)]
#[allow(missing_copy_implementations)]
#[non_exhaustive]
pub enum S3ConditionalPut {
    /// Some S3-compatible stores, such as Cloudflare R2 and minio support conditional
    /// put using the standard [HTTP precondition] headers If-Match and If-None-Match
    ///
    /// Encoded as `etag` ignoring whitespace
    ///
    /// [HTTP precondition]: https://datatracker.ietf.org/doc/html/rfc9110#name-preconditions
    #[default]
    ETagMatch,

    /// Disable `conditional put`
    Disabled,
}

impl std::fmt::Display for S3ConditionalPut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ETagMatch => write!(f, "etag"),
            Self::Disabled => write!(f, "disabled"),
        }
    }
}

impl S3ConditionalPut {
    fn from_str(s: &str) -> Option<Self> {
        match s.trim() {
            "etag" => Some(Self::ETagMatch),
            "disabled" => Some(Self::Disabled),
            _ => None,
        }
    }
}

impl Parse for S3ConditionalPut {
    fn parse(v: &str) -> crate::Result<Self> {
        Self::from_str(v).ok_or_else(|| crate::Error::Generic {
            store: "Config",
            source: format!("Failed to parse \"{v}\" as S3PutConditional").into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::S3CopyIfNotExists;

    #[test]
    fn parse_s3_copy_if_not_exists_header() {
        let input = "header: cf-copy-destination-if-none-match: *";
        let expected = Some(S3CopyIfNotExists::Header(
            "cf-copy-destination-if-none-match".to_owned(),
            "*".to_owned(),
        ));

        assert_eq!(expected, S3CopyIfNotExists::from_str(input));
    }

    #[test]
    fn parse_s3_copy_if_not_exists_header_with_status() {
        let input = "header-with-status:key:value:403";
        let expected = Some(S3CopyIfNotExists::HeaderWithStatus(
            "key".to_owned(),
            "value".to_owned(),
            http::StatusCode::FORBIDDEN,
        ));

        assert_eq!(expected, S3CopyIfNotExists::from_str(input));
    }

    #[test]
    fn parse_s3_copy_if_not_exists_header_whitespace_invariant() {
        let expected = Some(S3CopyIfNotExists::Header(
            "cf-copy-destination-if-none-match".to_owned(),
            "*".to_owned(),
        ));

        const INPUTS: &[&str] = &[
            "header:cf-copy-destination-if-none-match:*",
            "header: cf-copy-destination-if-none-match:*",
            "header: cf-copy-destination-if-none-match: *",
            "header : cf-copy-destination-if-none-match: *",
            "header : cf-copy-destination-if-none-match : *",
            "header : cf-copy-destination-if-none-match : * ",
        ];

        for input in INPUTS {
            assert_eq!(expected, S3CopyIfNotExists::from_str(input));
        }
    }

    #[test]
    fn parse_s3_copy_if_not_exists_header_with_status_whitespace_invariant() {
        let expected = Some(S3CopyIfNotExists::HeaderWithStatus(
            "key".to_owned(),
            "value".to_owned(),
            http::StatusCode::FORBIDDEN,
        ));

        const INPUTS: &[&str] = &[
            "header-with-status:key:value:403",
            "header-with-status: key:value:403",
            "header-with-status: key: value:403",
            "header-with-status: key: value: 403",
            "header-with-status : key: value: 403",
            "header-with-status : key : value: 403",
            "header-with-status : key : value : 403",
            "header-with-status : key : value : 403 ",
        ];

        for input in INPUTS {
            assert_eq!(expected, S3CopyIfNotExists::from_str(input));
        }
    }
}
