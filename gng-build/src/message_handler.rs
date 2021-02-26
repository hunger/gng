// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object used to handle messages from `gng-build-agent`

use gng_build_shared::SourcePacket;

use eyre::{eyre, Result, WrapErr};
use sha3::{Digest, Sha3_256};

// - Helper:
// ----------------------------------------------------------------------

fn hash_str(input: &str) -> Vec<u8> {
    let mut hasher = Sha3_256::new();
    hasher.update(input.as_bytes());
    let mut v = Vec::with_capacity(Sha3_256::output_size());
    v.extend_from_slice(&hasher.finalize());

    v
}

// ----------------------------------------------------------------------
// - Message Handler:
// ----------------------------------------------------------------------

/// An object used to handle messages from the `gng-build-agent`
pub trait MessageHandler {
    /// Verify state before `gng-build-agent` is started
    ///
    /// # Errors
    /// Generic Error
    fn prepare(&mut self, mode: &crate::Mode) -> Result<()>;

    /// Handle one message from `gng-build-agent`
    ///
    /// # Errors
    /// Generic Error
    fn handle(
        &mut self,
        mode: &crate::Mode,
        message_type: &gng_build_shared::MessageType,
        message: &str,
    ) -> Result<bool>;

    /// Verify state after `gng-build-agent` has quit successfully
    ///
    /// # Errors
    /// Generic Error
    fn verify(&mut self, mode: &crate::Mode) -> Result<()>;
}

// ----------------------------------------------------------------------
// - ImmutableSourceDataHandler:
// ----------------------------------------------------------------------

/// Make sure the source as seen by the `gng-build-agent` stays constant
#[derive(Debug)]
pub struct ImmutableSourceDataHandler {
    hash: Option<Vec<u8>>,
    first_message: bool,
}

impl Default for ImmutableSourceDataHandler {
    fn default() -> Self {
        Self {
            hash: None,
            first_message: true,
        }
    }
}

impl MessageHandler for ImmutableSourceDataHandler {
    #[tracing::instrument(level = "trace")]
    fn prepare(&mut self, mode: &crate::Mode) -> Result<()> {
        self.first_message = true;
        Ok(())
    }

    #[tracing::instrument(level = "trace")]
    fn handle(
        &mut self,
        mode: &crate::Mode,
        message_type: &gng_build_shared::MessageType,
        message: &str,
    ) -> Result<bool> {
        if message_type != &gng_build_shared::MessageType::Data {
            self.first_message = false;
            return Ok(false);
        }

        if !self.first_message {
            tracing::error!("The build agent did not send a DATA message first!");
            panic!("gng-build-agent did not react as expected!");
        }

        self.first_message = false;

        let v = hash_str(message);

        match self.hash.as_ref() {
            None => {
                self.hash = Some(v);
                Ok(false)
            }
            Some(vg) if *vg == v => Ok(false),
            Some(_) => {
                tracing::error!("Source data changed, aborting!");
                panic!("gng-build-agent did not react as expected!");
            }
        }
    }

    #[tracing::instrument(level = "trace")]
    fn verify(&mut self, mode: &crate::Mode) -> Result<()> {
        if self.first_message {
            tracing::error!("The build agent did not send any message!");
            panic!("gng-build-agent did not react as expected!");
        }

        if self.hash.is_none() {
            tracing::error!("No source data received during Query mode.");
            panic!("gng-build-agent did not react as expected!");
        }
        Ok(())
    }
}

// ----------------------------------------------------------------------
// - SourceHandler:
// ----------------------------------------------------------------------

struct UrlNormalizer {
    base_url: Option<url::Url>,
    base_directory: std::path::PathBuf,
    seen_urls: std::collections::HashSet<url::Url>,
}

impl UrlNormalizer {
    fn new(base_directory: &std::path::Path) -> Result<Self> {
        let base_url = Some(
            url::Url::parse(&format!("file://{}/", base_directory.to_string_lossy()))
                .wrap_err("Failed to PKGSRC directory into an URL.")?,
        );

        Ok(Self {
            base_url,
            base_directory: base_directory.to_owned(),
            seen_urls: std::collections::HashSet::<url::Url>::default(),
        })
    }

    fn normalize(&mut self, url: &str) -> Result<url::Url> {
        let source_url = if let Some(bu) = &self.base_url {
            bu.join(url).wrap_err("Failed to parse URL")?
        } else {
            url::Url::parse(url)?
        };

        if !self.seen_urls.insert(source_url.clone()) {
            return Err(eyre!("URL has been seen before in the same sources list"));
        }

        if source_url.scheme() == "file" {
            let url_file_name = std::fs::canonicalize(source_url.path())
                .wrap_err("Failed to canonicalize file path in source URL")?;

            if !url_file_name.starts_with(&self.base_directory) {
                return Err(eyre!(
                    "File URL is not pointing into the directory containing \"{}\".",
                    gng_build_shared::BUILD_SCRIPT
                ));
            }
            return Ok(source_url);
        }
        if source_url.scheme() == "http" || source_url.scheme() == "https" {
            return Ok(source_url);
        }

        Err(eyre!("Unsupported URL scheme."))
    }
}

/// Make sure the source as seen by the `gng-build-agent` stays constant
pub struct SourceHandler {
    sources: Vec<gng_build_shared::Source>,
    pkgsrc_directory: std::path::PathBuf,
    work_directory: std::path::PathBuf,
}

impl SourceHandler {
    fn new(pkgsrc_directory: &std::path::Path, work_directory: &std::path::Path) -> Self {
        Self {
            sources: Vec::new(),
            pkgsrc_directory: pkgsrc_directory.to_owned(),
            work_directory: work_directory.to_owned(),
        }
    }

    fn store_sources(&mut self, _source_packet: SourcePacket) -> Result<()> {
        let mut _normalizer = UrlNormalizer::new(&self.pkgsrc_directory)?;

        // for s in source_packet.sources {
        //     let source_url = normalizer.normalize(&s.url)?;
        //     if !s.name.chars().all(|c| {
        //         ('a'..='z').contains(&c)
        //             || ('A'..='Z').contains(&c)
        //             || ('0'..='9').contains(&c)
        //             || (c == '_')
        //             || (c == '-')
        //     }) {
        //         return Err(eyre!(format!(
        //             "Name for source {} contains invalid characters.",
        //             s
        //         )));
        //     };
        //     let source = gng_build_shared::Source {
        //         url: source_url.to_string(),
        //         ..s
        //     };
        //     self.sources.push(source);
        // }
        Ok(())
    }

    fn install_sources(&mut self) -> Result<()> {
        Ok(())
    }
}

impl MessageHandler for SourceHandler {
    fn prepare(&mut self, mode: &crate::Mode) -> Result<()> {
        if *mode == crate::Mode::Prepare {
            self.install_sources()
        } else {
            Ok(())
        }
    }

    fn handle(
        &mut self,
        mode: &crate::Mode,
        message_type: &gng_build_shared::MessageType,
        message: &str,
    ) -> Result<bool> {
        if *mode != crate::Mode::Query && message_type != &gng_build_shared::MessageType::Data {
            return Ok(false);
        }

        self.store_sources(serde_json::from_str(message).map_err(|e| eyre!(e))?)?;

        Ok(false)
    }

    fn verify(&mut self, _mode: &crate::Mode) -> Result<()> {
        assert!(!self.sources.is_empty());

        todo!();
    }
}

// ----------------------------------------------------------------------
// - PackageHandler:
// ----------------------------------------------------------------------

/// Make sure the source as seen by the `gng-build-agent` stays constant
pub struct PacketHandler {
    source_packet: Option<SourcePacket>,
}

impl Default for PacketHandler {
    fn default() -> Self {
        Self {
            source_packet: None,
        }
    }
}

impl MessageHandler for PacketHandler {
    fn prepare(&mut self, _mode: &crate::Mode) -> Result<()> {
        Ok(())
    }

    fn handle(
        &mut self,
        mode: &crate::Mode,
        message_type: &gng_build_shared::MessageType,
        _message: &str,
    ) -> Result<bool> {
        if *mode != crate::Mode::Query && message_type != &gng_build_shared::MessageType::Data {
            return Ok(false);
        }

        // let source_packet = serde_json::from_str(message).map_err(|e| eyre!(e))?;

        Ok(false)
    }

    fn verify(&mut self, _mode: &crate::Mode) -> Result<()> {
        assert!(self.source_packet.is_some());

        todo!();
    }
}

// ----------------------------------------------------------------------
// - Tests:
// ----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_immutable_source_data_handler_ok() {
        let mut handler = ImmutableSourceDataHandler::default();

        let mut mode = Some(crate::Mode::Query);
        while let Some(m) = crate::Mode::next(mode.unwrap()) {
            handler.prepare(&m).unwrap();
            handler
                .handle(&m, &gng_build_shared::MessageType::Data, "foobar 12345")
                .unwrap();
            handler.verify(&m).unwrap();
            mode = Some(m)
        }
    }
    #[test]
    fn test_immutable_source_data_handler_ok_data_same() {
        let mut handler = ImmutableSourceDataHandler::default();

        handler.prepare(&crate::Mode::Prepare).unwrap();
        handler
            .handle(
                &crate::Mode::Prepare,
                &gng_build_shared::MessageType::Data,
                "foobar 12345",
            )
            .unwrap();
        handler.verify(&crate::Mode::Prepare).unwrap();

        handler.prepare(&crate::Mode::Query).unwrap();
        handler
            .handle(
                &crate::Mode::Query,
                &gng_build_shared::MessageType::Data,
                "foobar 12345",
            )
            .unwrap();
        handler.verify(&crate::Mode::Query).unwrap();
    }

    #[test]
    #[should_panic(expected = "gng-build-agent did not react as expected!")]
    fn test_immutable_source_data_handler_no_data_message() {
        let mut handler = ImmutableSourceDataHandler::default();

        handler.prepare(&crate::Mode::Prepare).unwrap();
        handler.verify(&crate::Mode::Prepare).unwrap();
    }

    #[test]
    #[should_panic(expected = "gng-build-agent did not react as expected!")]
    fn test_immutable_source_data_handler_double_data() {
        let mut handler = ImmutableSourceDataHandler::default();

        handler.prepare(&crate::Mode::Prepare).unwrap();
        handler
            .handle(
                &crate::Mode::Prepare,
                &gng_build_shared::MessageType::Data,
                "foobar 12345",
            )
            .unwrap();
        handler
            .handle(
                &crate::Mode::Prepare,
                &gng_build_shared::MessageType::Data,
                "foobar 12345",
            )
            .unwrap();
        handler.verify(&crate::Mode::Prepare).unwrap();
    }

    #[test]
    #[should_panic(expected = "gng-build-agent did not react as expected!")]
    fn test_immutable_source_data_handler_non_data() {
        let mut handler = ImmutableSourceDataHandler::default();

        handler.prepare(&crate::Mode::Prepare).unwrap();
        handler
            .handle(
                &crate::Mode::Prepare,
                &gng_build_shared::MessageType::Test,
                "foobar 12345",
            )
            .unwrap();
        handler.verify(&crate::Mode::Prepare).unwrap();
    }

    #[test]
    #[should_panic(expected = "gng-build-agent did not react as expected!")]
    fn test_immutable_source_data_handler_data_changed() {
        let mut handler = ImmutableSourceDataHandler::default();

        handler.prepare(&crate::Mode::Prepare).unwrap();
        handler
            .handle(
                &crate::Mode::Prepare,
                &gng_build_shared::MessageType::Data,
                "foobar 12345",
            )
            .unwrap();
        handler.verify(&crate::Mode::Prepare).unwrap();

        handler.prepare(&crate::Mode::Query).unwrap();
        handler
            .handle(
                &crate::Mode::Query,
                &gng_build_shared::MessageType::Data,
                "foobar 123456",
            )
            .unwrap();
        handler.verify(&crate::Mode::Query).unwrap();
    }

    #[test]
    fn test_url_normalizer_ok() {
        let mut normalizer = UrlNormalizer::new(std::path::Path::new("/tmp/testing")).unwrap();
        assert_eq!(
            normalizer
                .normalize("http://google.com/index.html")
                .unwrap()
                .to_string(),
            "http://google.com/index.html"
        );
        assert_eq!(
            normalizer
                .normalize("https://google.com/index.html")
                .unwrap()
                .to_string(),
            "https://google.com/index.html"
        );
    }

    #[test]
    fn test_url_normalizer_not_ok() {
        let mut normalizer = UrlNormalizer::new(std::path::Path::new("/tmp/testing")).unwrap();

        assert!(normalizer.normalize("ftp://google.com/index.html").is_err());
    }
}
