/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Utilities for working with the `tracing` crate.

use std::sync::Once;

use anstream::stderr;
use tracing_subscriber::Layer;
use tracing_subscriber::Registry;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub type TracingLayer = Box<dyn Layer<Registry> + Send + Sync>;

/// Create a layer for user tracing.
pub fn tracing_layer(verbose: bool, testing: bool) -> TracingLayer {
    const ENV_VAR: &str = "PYREFLY_LOG";
    let mut env_filter = EnvFilter::from_env(ENV_VAR);
    if std::env::var_os(ENV_VAR).is_none() {
        // Enable info log by default
        env_filter = env_filter.add_directive(if verbose {
            LevelFilter::DEBUG.into()
        } else {
            LevelFilter::INFO.into()
        });
    }

    let layer = tracing_subscriber::fmt::layer()
        .with_line_number(false)
        .with_file(false)
        .without_time()
        .with_writer(stderr)
        .with_target(false);

    if testing {
        // The with_test_writer causes us to write to stdout, not stderr.
        layer.with_test_writer().with_filter(env_filter).boxed()
    } else {
        layer.with_filter(env_filter).boxed()
    }
}

/// Set up tracing so it prints to stderr, and can be used for output.
/// Most things should use `info` and `debug` level for showing messages.
pub fn init_tracing(verbose: bool, testing: bool) {
    // If we create tracing twice, the library panics. Avoid that.
    // Mostly happens when we run tests.
    static INIT_TRACING_ONCE: Once = Once::new();

    INIT_TRACING_ONCE.call_once(|| {
        tracing_subscriber::registry()
            .with(tracing_layer(verbose, testing))
            .init();
    })
}
