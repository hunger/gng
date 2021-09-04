// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object used to handle events from `gng-build-agent` received via the `CaseOfficer`

use eyre::Result;

// - Helper:
// ----------------------------------------------------------------------

// fn packet_from(source_package: &gng_build_shared::SourcePacket) -> gng_core::PacketFileDataBuilder {
//     let mut result = gng_core::PacketFileDataBuilder::default();
//     result.source_name(source_package.name.clone());
//     result.license(source_package.license.clone());
//     result.version(source_package.version.clone());
//     result.url(source_package.url.clone());
//     result.bug_url(source_package.bug_url.clone());
//     result
// }

// #[tracing::instrument(level = "trace", skip(source_package, ctx))]
// fn package(
//     source_package: &gng_build_shared::SourcePacket,
//     ctx: &crate::handler::Context,
// ) -> Result<Vec<crate::packager::PacketFiles>> {
//     let mut packager = crate::PackagerBuilder::default();

//     let mut has_base_packet = false;

//     for pd in &source_package.packets {
//         if pd.name == source_package.name {
//             has_base_packet = true
//         }

//         let p = packet_from(source_package)
//             .name(pd.name.clone())
//             .register_facet(pd.facet.clone())
//             .description(pd.description.clone())
//             // FIXME: Handle dependencies!
//             // .dependencies(pd.dependencies.clone())
//             .build()
//             .map_err(|e| gng_core::Error::Runtime {
//                 message: format!("Failed to define a packet: {}", e),
//             })?;
//         let patterns = pd
//             .files
//             .iter()
//             .map(|d| {
//                 glob::Pattern::new(&d.to_string())
//                     .wrap_err("Failed to convert packet files to glob patterns.")
//             })
//             .collect::<Result<Vec<glob::Pattern>>>()?;

//         let contents_policy = if patterns.is_empty() {
//             crate::ContentsPolicy::Empty
//         } else {
//             crate::ContentsPolicy::NotEmpty
//         };

//         packager = packager.add_packet(&p, &patterns[..], contents_policy)?;
//     }

//     if !has_base_packet {
//         let p = packet_from(source_package)
//             .name(source_package.name.clone())
//             .description(source_package.description.clone())
//             .dependencies(Vec::new())
//             .register_facet(None)
//             .build()
//             .map_err(|e| gng_core::Error::Runtime {
//                 message: format!("Failed to define a packet: {}", e),
//             })?;

//         packager = packager.add_packet(
//             &p,
//             &[glob::Pattern::new("**").wrap_err("Failed to register catch-all glob pattern.")?],
//             crate::ContentsPolicy::MaybeEmpty,
//         )?;
//     }

//     packager
//         .build()?
//         .package(&ctx.install_directory, &std::env::current_dir()?)
//         .wrap_err(format!(
//             "Failed to package \"{}\".",
//             ctx.install_directory.to_string_lossy()
//         ))
// }

// ----------------------------------------------------------------------
// - Context:
// ----------------------------------------------------------------------

/// A `Context` in which a `Handler` is run
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

// // ----------------------------------------------------------------------
// // - SourcePacketInfo:
// // ----------------------------------------------------------------------

// /// A simple struct to provide access to the `SourcePacket` to handlers that need to care.
// pub struct SourcePacketInfo {
//     source_packet: std::cell::RefCell<Option<gng_build_shared::SourcePacket>>,
// }

// impl SourcePacketInfo {
//     /// Get the `SourcePacket`
//     ///
//     /// # Errors
//     /// Errors out if the `SourcePacket` has not been set yet!
//     pub fn get(&self) -> Result<gng_build_shared::SourcePacket> {
//         self.source_packet
//             .borrow()
//             .clone()
//             .ok_or_else(|| eyre::eyre!("SourcePacket has not been set yet!"))
//     }

//     /// Set the `SourcePacket`
//     ///
//     /// # Errors
//     /// Errors out if the `SourcePacket` has been set before!
//     pub fn set(&self, source_packet: gng_build_shared::SourcePacket) -> Result<()> {
//         let mut sp = self.source_packet.borrow_mut();
//         if sp.is_some() {
//             Err(eyre::eyre!("SourcePacket has already been set!"))
//         } else {
//             *sp = Some(source_packet);
//             Ok(())
//         }
//     }
// }

// impl Default for SourcePacketInfo {
//     fn default() -> Self {
//         Self {
//             source_packet: std::cell::RefCell::new(None),
//         }
//     }
// }

// ----------------------------------------------------------------------
// - Handler:
// ----------------------------------------------------------------------

/// An object used to handle events from the `gng-build-agent`
pub trait Handler {
    /// Verify state before `gng-build-agent` is started
    ///
    /// # Errors
    /// Generic Error
    fn prepare(&mut self, _mode: &crate::Mode) -> Result<()> {
        Ok(())
    }

    /// Handle one message from `gng-build-agent`
    ///
    /// Return `Ok(true)` if this handler handled the message and it does
    /// not need to get passed on to other handlers.
    ///
    /// # Errors
    /// Generic Error
    fn handle(
        &mut self,
        _mode: &crate::Mode,
        _message_type: &gng_build_shared::MessageType,
        _message: &str,
    ) -> Result<bool> {
        Ok(false)
    }

    /// Verify state after `gng-build-agent` has quit successfully
    ///
    /// # Errors
    /// Generic Error
    fn verify(&mut self, _mode: &crate::Mode) -> Result<()> {
        Ok(())
    }
}

// // ----------------------------------------------------------------------
// // - ImmutableSourceDataHandler:
// // ----------------------------------------------------------------------

// /// Make sure the source as seen by the `gng-build-agent` stays constant
// pub struct ImmutableSourceDataHandler {
//     hash: Option<gng_core::Hash>,
//     first_message: bool,
// }

// impl Default for ImmutableSourceDataHandler {
//     fn default() -> Self {
//         Self {
//             hash: None,
//             first_message: true,
//         }
//     }
// }

// impl Handler for ImmutableSourceDataHandler {
//     #[tracing::instrument(level = "trace", skip(self, _ctx))]
//     fn prepare(&mut self, _ctx: &Context, mode: &crate::Mode) -> Result<()> {
//         self.first_message = true;
//         Ok(())
//     }

//     #[tracing::instrument(level = "trace", skip(self, _ctx))]
//     fn handle(
//         &mut self,
//         _ctx: &Context,
//         mode: &crate::Mode,
//         message_type: &gng_build_shared::MessageType,
//         message: &str,
//     ) -> Result<bool> {
//         if message_type != &gng_build_shared::MessageType::Data {
//             self.first_message = false;
//             return Ok(false);
//         }

//         if !self.first_message {
//             panic!("gng-build-agent did not react as expected!");
//         }

//         self.first_message = false;

//         let v = gng_core::Hash::calculate_sha256(message.as_bytes());

//         match self.hash.as_ref() {
//             None => {
//                 self.hash = Some(v);
//             }
//             Some(vg) if *vg == v => {}
//             Some(_) => {
//                 panic!("gng-build-agent did not react as expected!");
//             }
//         }

//         Ok(false)
//     }

//     #[tracing::instrument(level = "trace", skip(self, _ctx))]
//     fn verify(&mut self, _ctx: &Context, mode: &crate::Mode) -> Result<()> {
//         if self.first_message {
//             panic!("gng-build-agent did not react as expected!");
//         }

//         if self.hash.is_none() {
//             panic!("gng-build-agent did not react as expected!");
//         }
//         Ok(())
//     }
// }

// // ----------------------------------------------------------------------
// // - ParseSourceDataHandler:
// // ----------------------------------------------------------------------

// /// Make sure the source as seen by the `gng-build-agent` stays constant
// pub struct ParseSourceDataHandler {
//     source_packet_info: std::rc::Rc<SourcePacketInfo>,
// }

// impl ParseSourceDataHandler {
//     /// Create a new `ImmutableSourceDataHandler`
//     pub const fn new(source_packet_info: std::rc::Rc<SourcePacketInfo>) -> Self {
//         Self { source_packet_info }
//     }
// }

// impl Handler for ParseSourceDataHandler {
//     #[tracing::instrument(level = "trace", skip(self, _ctx))]
//     fn handle(
//         &mut self,
//         _ctx: &Context,
//         mode: &crate::Mode,
//         message_type: &gng_build_shared::MessageType,
//         message: &str,
//     ) -> Result<bool> {
//         if *mode == crate::Mode::Query && message_type == &gng_build_shared::MessageType::Data {
//             let data: gng_build_shared::SourcePacket = serde_json::from_str(message)?;
//             self.source_packet_info.set(data)?;
//         }

//         Ok(false)
//     }
// }

// // ----------------------------------------------------------------------
// // - ValidateHandler:
// // ----------------------------------------------------------------------

// /// Make sure the source as seen by the `gng-build-agent` stays constant
// pub struct ValidateHandler {
//     source_packet_info: std::rc::Rc<SourcePacketInfo>,
// }

// impl ValidateHandler {
//     /// Create a new `ValidateHandler`.
//     pub const fn new(source_packet_info: std::rc::Rc<SourcePacketInfo>) -> Self {
//         Self { source_packet_info }
//     }
// }

// impl Handler for ValidateHandler {
//     #[tracing::instrument(level = "trace", skip(self, _ctx))]
//     fn verify(&mut self, _ctx: &Context, mode: &crate::Mode) -> Result<()> {
//         if *mode == crate::Mode::Query {
//             let source_packet = self.source_packet_info.get()?;
//             let build_dependencies = source_packet.build_dependencies.clone();
//             for p in &source_packet.packets {
//                 for pd in &p.dependencies {
//                     if !build_dependencies.contains(pd) {
//                         tracing::error!("Packet \"{}\" has a dependency \"{}\" that is not a build dependency of the Source Packet.", &p.name, pd);
//                         return Err(eyre::eyre!("Packet \"{}\" has a dependency \"{}\" that is not a build dependency of the Source Packet.", &p.name, pd));
//                     }
//                 }
//             }
//         }
//         Ok(())
//     }
// }

// // ----------------------------------------------------------------------
// // - PackagingHandler:
// // ----------------------------------------------------------------------

// /// Make sure the source as seen by the `gng-build-agent` stays constant
// pub struct PackagingHandler {
//     source_packet_info: std::rc::Rc<SourcePacketInfo>,
// }

// impl PackagingHandler {
//     /// Create a new `PackagingHandler`.
//     pub const fn new(source_packet_info: std::rc::Rc<SourcePacketInfo>) -> Self {
//         Self { source_packet_info }
//     }
// }

// impl Handler for PackagingHandler {
//     #[tracing::instrument(level = "trace", skip(self, ctx))]
//     fn verify(&mut self, ctx: &Context, mode: &crate::Mode) -> Result<()> {
//         if *mode != crate::Mode::Package {
//             return Ok(());
//         }

//         let source_packet = self.source_packet_info.get()?;
//         let result = package(&source_packet, ctx)?;
//         for r in &result {
//             let packet_name = &r.packet.name;
//             for f in &r.files {
//                 println!("{}: {} - {}", packet_name, &f.0.display(), &f.1);
//             }
//         }
//         Ok(())
//     }
// }

// // ----------------------------------------------------------------------
// // - Tests:
// // ----------------------------------------------------------------------

// #[cfg(test)]
// mod tests {
//     use super::*;

//     fn create_ctx() -> Context {
//         let tmp = std::path::PathBuf::from(".");
//         Context {
//             lua_directory: tmp.clone(),
//             work_directory: tmp.clone(),
//             install_directory: tmp.clone(),
//             build_file: tmp.clone(),
//             build_agent: tmp,
//         }
//     }
//     #[test]
//     fn immutable_source_data_handler_ok() {
//         let mut handler = ImmutableSourceDataHandler::default();

//         let ctx = create_ctx();

//         let mut mode = Some(crate::Mode::Query);
//         while let Some(m) = crate::Mode::next(mode.unwrap()) {
//             handler.prepare(&ctx, &m).unwrap();
//             handler
//                 .handle(
//                     &ctx,
//                     &m,
//                     &gng_build_shared::MessageType::Data,
//                     "foobar 12345",
//                 )
//                 .unwrap();
//             handler.verify(&ctx, &m).unwrap();
//             mode = Some(m)
//         }
//     }
//     #[test]
//     fn immutable_source_data_handler_ok_data_same() {
//         let mut handler = ImmutableSourceDataHandler::default();

//         let ctx = create_ctx();

//         handler.prepare(&ctx, &crate::Mode::Prepare).unwrap();
//         handler
//             .handle(
//                 &ctx,
//                 &crate::Mode::Prepare,
//                 &gng_build_shared::MessageType::Data,
//                 "foobar 12345",
//             )
//             .unwrap();
//         handler.verify(&ctx, &crate::Mode::Prepare).unwrap();

//         handler.prepare(&ctx, &crate::Mode::Query).unwrap();
//         handler
//             .handle(
//                 &ctx,
//                 &crate::Mode::Query,
//                 &gng_build_shared::MessageType::Data,
//                 "foobar 12345",
//             )
//             .unwrap();
//         handler.verify(&ctx, &crate::Mode::Query).unwrap();
//     }

//     #[test]
//     #[should_panic(expected = "gng-build-agent did not react as expected!")]
//     fn immutable_source_data_handler_no_data_message() {
//         let mut handler = ImmutableSourceDataHandler::default();

//         let ctx = create_ctx();

//         handler.prepare(&ctx, &crate::Mode::Prepare).unwrap();
//         handler.verify(&ctx, &crate::Mode::Prepare).unwrap();
//     }

//     #[test]
//     #[should_panic(expected = "gng-build-agent did not react as expected!")]
//     fn immutable_source_data_handler_double_data() {
//         let mut handler = ImmutableSourceDataHandler::default();

//         let ctx = create_ctx();

//         handler.prepare(&ctx, &crate::Mode::Prepare).unwrap();
//         handler
//             .handle(
//                 &ctx,
//                 &crate::Mode::Prepare,
//                 &gng_build_shared::MessageType::Data,
//                 "foobar 12345",
//             )
//             .unwrap();
//         handler
//             .handle(
//                 &ctx,
//                 &crate::Mode::Prepare,
//                 &gng_build_shared::MessageType::Data,
//                 "foobar 12345",
//             )
//             .unwrap();
//         handler.verify(&ctx, &crate::Mode::Prepare).unwrap();
//     }

//     #[test]
//     #[should_panic(expected = "gng-build-agent did not react as expected!")]
//     fn immutable_source_data_handler_non_data() {
//         let mut handler = ImmutableSourceDataHandler::default();

//         let ctx = create_ctx();

//         handler.prepare(&ctx, &crate::Mode::Prepare).unwrap();
//         handler
//             .handle(
//                 &ctx,
//                 &crate::Mode::Prepare,
//                 &gng_build_shared::MessageType::Test,
//                 "foobar 12345",
//             )
//             .unwrap();
//         handler.verify(&ctx, &crate::Mode::Prepare).unwrap();
//     }

//     #[test]
//     #[should_panic(expected = "gng-build-agent did not react as expected!")]
//     fn immutable_source_data_handler_data_changed() {
//         let mut handler = ImmutableSourceDataHandler::default();

//         let ctx = create_ctx();

//         handler.prepare(&ctx, &crate::Mode::Prepare).unwrap();
//         handler
//             .handle(
//                 &ctx,
//                 &crate::Mode::Prepare,
//                 &gng_build_shared::MessageType::Data,
//                 "foobar 12345",
//             )
//             .unwrap();
//         handler.verify(&ctx, &crate::Mode::Prepare).unwrap();

//         handler.prepare(&ctx, &crate::Mode::Query).unwrap();
//         handler
//             .handle(
//                 &ctx,
//                 &crate::Mode::Query,
//                 &gng_build_shared::MessageType::Data,
//                 "foobar 123456",
//             )
//             .unwrap();
//         handler.verify(&ctx, &crate::Mode::Query).unwrap();
//     }
// }
