// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

use log::LevelFilter;

/// Crates whose messages follow the configured log level. Everything else is a
/// dependency and is capped at DEP_MAX_LEVEL.
pub const OWN_TARGETS: [&str; 2] = ["bronze_monkey", "retouched_server"];

pub const DEP_MAX_LEVEL: LevelFilter = LevelFilter::Warn;

pub fn is_own_target(target: &str) -> bool {
    OWN_TARGETS.iter().any(|own| target.starts_with(own))
}
