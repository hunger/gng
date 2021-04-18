// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object used to handle events from the `CaseOfficer` from `gng-build-agent`

use eyre::{Result, WrapErr};
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

fn packet_from(source_package: &gng_build_shared::SourcePacket) -> gng_shared::PacketBuilder {
    let mut result = gng_shared::PacketBuilder::default();
    result.source_name(source_package.name.clone());
    result.license(source_package.license.clone());
    result.version(source_package.version.clone());
    result.url(source_package.url.clone());
    result.bug_url(source_package.bug_url.clone());
    result
}

#[tracing::instrument(level = "trace", skip(source_package))]
fn package(
    source_package: &gng_build_shared::SourcePacket,
    ctx: &crate::handler::Context,
) -> Result<Vec<std::path::PathBuf>> {
    let mut packager = crate::PackagerBuilder::default();

    let mut has_base_packet = false;

    for pd in &source_package.packets {
        if pd.name == source_package.name {
            has_base_packet = true
        }

        let p = packet_from(source_package)
            .name(pd.name.clone())
            .facet(pd.facet.clone())
            .description(pd.description.clone())
            .dependencies(pd.dependencies.clone())
            .build()
            .map_err(|e| gng_shared::Error::Runtime {
                message: format!("Failed to define a packet: {}", e),
            })?;
        let patterns = pd
            .files
            .iter()
            .map(|d| {
                glob::Pattern::new(&d.to_string())
                    .wrap_err("Failed to convert packet files to glob patterns.")
            })
            .collect::<Result<Vec<glob::Pattern>>>()?;

        let contents_policy = if patterns.is_empty() {
            crate::ContentsPolicy::Empty
        } else {
            crate::ContentsPolicy::NotEmpty
        };

        packager = packager.add_packet(&p, &patterns[..], contents_policy)?;
    }

    if !has_base_packet {
        let p = packet_from(source_package)
            .name(source_package.name.clone())
            .description(source_package.description.clone())
            .dependencies(gng_shared::Names::default())
            .facet(None)
            .build()
            .map_err(|e| gng_shared::Error::Runtime {
                message: format!("Failed to define a packet: {}", e),
            })?;

        packager = packager.add_packet(
            &p,
            &[glob::Pattern::new("**").wrap_err("Failed to register catch-all glob pattern.")?],
            crate::ContentsPolicy::MaybeEmpty,
        )?;
    }

    packager
        .build()?
        .package(&ctx.install_directory, &std::env::current_dir()?)
        .wrap_err(format!(
            "Failed to package \"{}\".",
            ctx.install_directory.to_string_lossy()
        ))
}

// ----------------------------------------------------------------------
// - Context:
// ----------------------------------------------------------------------

/// A `Context` in which a `Handler` is run
#[derive(Debug)]
pub struct Context {
    /// The Lua directory with additional Lua files.
    pub lua_directory: std::path::PathBuf,
    /// The directory the build script can work in
    pub work_directory: std::path::PathBuf,
    /// The directory the build script will install into
    pub install_directory: std::path::PathBuf,

    /// The actual build file that is being used
    pub build_file: std::path::PathBuf,
    /// The build agent that is being used
    pub build_agent: std::path::PathBuf,
}

// ----------------------------------------------------------------------
// - Handler:
// ----------------------------------------------------------------------

/// An object used to handle events from the `gng-build-agent`
pub trait Handler {
    /// Verify state before `gng-build-agent` is started
    ///
    /// # Errors
    /// Generic Error
    fn prepare(&mut self, ctx: &Context, mode: &crate::Mode) -> Result<()>;

    /// Handle one message from `gng-build-agent`
    ///
    /// Return `Ok(true)` if this handler handled the message and it does
    /// not need to get passed on to other handlers.
    ///
    /// # Errors
    /// Generic Error
    fn handle(
        &mut self,
        ctx: &Context,
        mode: &crate::Mode,
        message_type: &gng_build_shared::MessageType,
        message: &str,
    ) -> Result<bool>;

    /// Verify state after `gng-build-agent` has quit successfully
    ///
    /// # Errors
    /// Generic Error
    fn verify(&mut self, ctx: &Context, mode: &crate::Mode) -> Result<()>;
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

impl Handler for ImmutableSourceDataHandler {
    #[tracing::instrument(level = "trace")]
    fn prepare(&mut self, ctx: &Context, mode: &crate::Mode) -> Result<()> {
        self.first_message = true;
        Ok(())
    }

    #[tracing::instrument(level = "trace")]
    fn handle(
        &mut self,
        ctx: &Context,
        mode: &crate::Mode,
        message_type: &gng_build_shared::MessageType,
        message: &str,
    ) -> Result<bool> {
        if message_type != &gng_build_shared::MessageType::Data {
            self.first_message = false;
            return Ok(false);
        }

        if !self.first_message {
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
                panic!("gng-build-agent did not react as expected!");
            }
        }
    }

    #[tracing::instrument(level = "trace")]
    fn verify(&mut self, ctx: &Context, mode: &crate::Mode) -> Result<()> {
        if self.first_message {
            panic!("gng-build-agent did not react as expected!");
        }

        if self.hash.is_none() {
            panic!("gng-build-agent did not react as expected!");
        }
        Ok(())
    }
}

// ----------------------------------------------------------------------
// - ValidatePacketsHandler:
// ----------------------------------------------------------------------

/// Make sure the source as seen by the `gng-build-agent` stays constant
#[derive(Debug)]
pub struct ValidatePacketsHandler {}

impl Default for ValidatePacketsHandler {
    fn default() -> Self {
        Self {}
    }
}

impl Handler for ValidatePacketsHandler {
    #[tracing::instrument(level = "trace")]
    fn prepare(&mut self, ctx: &Context, mode: &crate::Mode) -> Result<()> {
        Ok(())
    }

    #[tracing::instrument(level = "trace")]
    fn handle(
        &mut self,
        ctx: &Context,
        mode: &crate::Mode,
        message_type: &gng_build_shared::MessageType,
        message: &str,
    ) -> Result<bool> {
        if *mode == crate::Mode::Query && message_type == &gng_build_shared::MessageType::Data {
            let data: gng_build_shared::SourcePacket = serde_json::from_str(message)?;

            let build_dependencies = data.build_dependencies.clone();
            for p in &data.packets {
                for pd in &p.dependencies {
                    if !build_dependencies.contains(pd) {
                        tracing::error!("Packet \"{}\" has a dependency \"{}\" that is not a build dependency of the Source Packet.", &p.name, pd);
                        return Err(eyre::eyre!("Packet \"{}\" has a dependency \"{}\" that is not a build dependency of the Source Packet.", &p.name, pd));
                    }
                }
            }
        }
        Ok(false)
    }

    #[tracing::instrument(level = "trace")]
    fn verify(&mut self, ctx: &Context, mode: &crate::Mode) -> Result<()> {
        Ok(())
    }
}

// ----------------------------------------------------------------------
// - PackagingHandler:
// ----------------------------------------------------------------------

/// Make sure the source as seen by the `gng-build-agent` stays constant
#[derive(Debug)]
pub struct PackagingHandler {
    source_packet: Option<gng_build_shared::SourcePacket>,
}

impl Default for PackagingHandler {
    fn default() -> Self {
        Self {
            source_packet: None,
        }
    }
}

impl Handler for PackagingHandler {
    #[tracing::instrument(level = "trace")]
    fn prepare(&mut self, ctx: &Context, mode: &crate::Mode) -> Result<()> {
        Ok(())
    }

    #[tracing::instrument(level = "trace")]
    fn handle(
        &mut self,
        ctx: &Context,
        mode: &crate::Mode,
        message_type: &gng_build_shared::MessageType,
        message: &str,
    ) -> Result<bool> {
        if *mode == crate::Mode::Query && message_type == &gng_build_shared::MessageType::Data {
            self.source_packet = serde_json::from_str(message)?;
        }
        Ok(false)
    }

    #[tracing::instrument(level = "trace")]
    fn verify(&mut self, ctx: &Context, mode: &crate::Mode) -> Result<()> {
        if *mode != crate::Mode::Package {
            return Ok(());
        }

        match &self.source_packet {
            Some(source_packet) => package(source_packet, ctx).map(|_| ()),
            None => Err(eyre::eyre!("Can not package: No data found!")),
        }
    }
}

// ----------------------------------------------------------------------
// - Tests:
// ----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn create_ctx() -> Context {
        let tmp = std::path::PathBuf::from(".");
        Context {
            lua_directory: tmp.clone(),
            work_directory: tmp.clone(),
            install_directory: tmp.clone(),
            build_file: tmp.clone(),
            build_agent: tmp,
        }
    }
    #[test]
    fn test_immutable_source_data_handler_ok() {
        let mut handler = ImmutableSourceDataHandler::default();

        let ctx = create_ctx();

        let mut mode = Some(crate::Mode::Query);
        while let Some(m) = crate::Mode::next(mode.unwrap()) {
            handler.prepare(&ctx, &m).unwrap();
            handler
                .handle(
                    &ctx,
                    &m,
                    &gng_build_shared::MessageType::Data,
                    "foobar 12345",
                )
                .unwrap();
            handler.verify(&ctx, &m).unwrap();
            mode = Some(m)
        }
    }
    #[test]
    fn test_immutable_source_data_handler_ok_data_same() {
        let mut handler = ImmutableSourceDataHandler::default();

        let ctx = create_ctx();

        handler.prepare(&ctx, &crate::Mode::Prepare).unwrap();
        handler
            .handle(
                &ctx,
                &crate::Mode::Prepare,
                &gng_build_shared::MessageType::Data,
                "foobar 12345",
            )
            .unwrap();
        handler.verify(&ctx, &crate::Mode::Prepare).unwrap();

        handler.prepare(&ctx, &crate::Mode::Query).unwrap();
        handler
            .handle(
                &ctx,
                &crate::Mode::Query,
                &gng_build_shared::MessageType::Data,
                "foobar 12345",
            )
            .unwrap();
        handler.verify(&ctx, &crate::Mode::Query).unwrap();
    }

    #[test]
    #[should_panic(expected = "gng-build-agent did not react as expected!")]
    fn test_immutable_source_data_handler_no_data_message() {
        let mut handler = ImmutableSourceDataHandler::default();

        let ctx = create_ctx();

        handler.prepare(&ctx, &crate::Mode::Prepare).unwrap();
        handler.verify(&ctx, &crate::Mode::Prepare).unwrap();
    }

    #[test]
    #[should_panic(expected = "gng-build-agent did not react as expected!")]
    fn test_immutable_source_data_handler_double_data() {
        let mut handler = ImmutableSourceDataHandler::default();
        let ctx = create_ctx();

        handler.prepare(&ctx, &crate::Mode::Prepare).unwrap();
        handler
            .handle(
                &ctx,
                &crate::Mode::Prepare,
                &gng_build_shared::MessageType::Data,
                "foobar 12345",
            )
            .unwrap();
        handler
            .handle(
                &ctx,
                &crate::Mode::Prepare,
                &gng_build_shared::MessageType::Data,
                "foobar 12345",
            )
            .unwrap();
        handler.verify(&ctx, &crate::Mode::Prepare).unwrap();
    }

    #[test]
    #[should_panic(expected = "gng-build-agent did not react as expected!")]
    fn test_immutable_source_data_handler_non_data() {
        let mut handler = ImmutableSourceDataHandler::default();
        let ctx = create_ctx();

        handler.prepare(&ctx, &crate::Mode::Prepare).unwrap();
        handler
            .handle(
                &ctx,
                &crate::Mode::Prepare,
                &gng_build_shared::MessageType::Test,
                "foobar 12345",
            )
            .unwrap();
        handler.verify(&ctx, &crate::Mode::Prepare).unwrap();
    }

    #[test]
    #[should_panic(expected = "gng-build-agent did not react as expected!")]
    fn test_immutable_source_data_handler_data_changed() {
        let mut handler = ImmutableSourceDataHandler::default();
        let ctx = create_ctx();

        handler.prepare(&ctx, &crate::Mode::Prepare).unwrap();
        handler
            .handle(
                &ctx,
                &crate::Mode::Prepare,
                &gng_build_shared::MessageType::Data,
                "foobar 12345",
            )
            .unwrap();
        handler.verify(&ctx, &crate::Mode::Prepare).unwrap();

        handler.prepare(&ctx, &crate::Mode::Query).unwrap();
        handler
            .handle(
                &ctx,
                &crate::Mode::Query,
                &gng_build_shared::MessageType::Data,
                "foobar 123456",
            )
            .unwrap();
        handler.verify(&ctx, &crate::Mode::Query).unwrap();
    }

    #[test]
    fn test_validate_packet_handler_ok() {
        let mut handler = ValidatePacketsHandler::default();
        let ctx = create_ctx();

        let mut mode = Some(crate::Mode::Query);
        while let Some(m) = crate::Mode::next(mode.unwrap()) {
            handler.prepare(&ctx, &m).unwrap();
            handler
                .handle(&ctx, &m, &gng_build_shared::MessageType::Data, r#"{"name":"filesystem","description":"Basic filesystem layout and facets","version":"1.0.0-1","license":"GPL-v3-or-later","url":null,"bug_url":null,"bootstrap":true,"build_dependencies":["foo"],"check_dependencies":[],"sources":[],"packets":[{"name":"dev","description":"Development files","dependencies":[],"files":[],"facet":{"description_suffix":"development files","mime_types":[],"patterns":["include/**"]}}]}"#)
                .unwrap();
            handler.verify(&ctx, &m).unwrap();
            mode = Some(m)
        }

        let mut mode = Some(crate::Mode::Query);
        while let Some(m) = crate::Mode::next(mode.unwrap()) {
            handler.prepare(&ctx, &m).unwrap();
            handler
                .handle(
                    &ctx,
                    &m,
                    &gng_build_shared::MessageType::Test,
                    r#"{"nXXX broken JSON"#,
                )
                .unwrap();
            handler.verify(&ctx, &m).unwrap();
            mode = Some(m)
        }
    }

    #[test]
    fn test_validate_packet_handler_err_wrong_dependencies() {
        let mut handler = ValidatePacketsHandler::default();
        let ctx = create_ctx();

        assert!(handler
            .handle(&ctx, &crate::Mode::Query, &gng_build_shared::MessageType::Data, r#"{"name":"filesystem","description":"Basic filesystem layout and facets","version":"1.0.0-1","license":"GPL-v3-or-later","url":null,"bug_url":null,"bootstrap":true,"build_dependencies":["foo"],"check_dependencies":[],"sources":[],"packets":[{"name":"dev","description":"Development files","dependencies":["bar"],"files":[],"facet":{"description_suffix":"development files","mime_types":[],"patterns":["include/**"]}}]}"#)
            .is_err());
    }
}
