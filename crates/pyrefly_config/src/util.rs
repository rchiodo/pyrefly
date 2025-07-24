/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use std::fmt::Display;
use std::ops::Deref;
use std::ops::DerefMut;

use serde::Deserialize;
use serde::Serialize;

/// Used in serde's skip_serializing_if attribute to skip serializing a boolean field that defaults to true.
#[allow(clippy::trivially_copy_pass_by_ref)]
pub(crate) fn skip_default_true(v: &bool) -> bool {
    *v
}

/// Used in serde's skip_serializing_if attribute to skip serializing a boolean field that defaults to true.
#[allow(clippy::trivially_copy_pass_by_ref)]
pub(crate) fn skip_default_false(v: &bool) -> bool {
    !*v
}

/// Used in serde's skip_serializing_if attribute to skip serializing a boolean field that defaults to false.
pub(crate) fn none_or_empty<T>(v: &Option<Vec<T>>) -> bool {
    v.as_ref().is_none_or(|v| v.is_empty())
}

/// A helper struct for detailing the origin of a config option.
/// The `Serialize` functionality enables us to only serialize [`ConfigOrigin::ConfigFile`]
/// values, skipping serialization for values we might be able to figure out automatically,
/// and we deserialize directly into a `ConfigFile` variant, making it easy to
/// update with a different `Auto` or `CommandLine` variant when overriding it.
#[derive(Debug, PartialEq, Eq, Deserialize, Clone, Copy)]
#[serde(untagged)]
pub(crate) enum ConfigOrigin<T> {
    /// This value was explicitly provided from a CLI flag.
    #[serde(skip)]
    CommandLine(T),

    /// This value was automatically constructed by Pyrefly.
    #[serde(skip)]
    Auto(T),

    /// This value was provided by a [`ConfigFile`], either explicitly or implicitly.
    ConfigFile(T),
}

impl<T: Default> Default for ConfigOrigin<T> {
    fn default() -> Self {
        ConfigOrigin::ConfigFile(T::default())
    }
}

impl<T: Serialize> Serialize for ConfigOrigin<T> {
    /// Serialize this `ConfigOrigin`'s internal value, making the `ConfigOrigin`
    /// transparent. This will serialize ALL `ConfigOrigin` values,
    /// so you must use `skip_serializing_if` helpers below to better control
    /// when to output a value.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.deref().serialize(serializer)
    }
}

impl<T: Display> Display for ConfigOrigin<T> {
    /// Display this `ConfigOrigin`'s internal value, making the `ConfigOrigin`
    /// transparent.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}

impl<T> Deref for ConfigOrigin<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::CommandLine(value) | Self::ConfigFile(value) | Self::Auto(value) => value,
        }
    }
}

impl<T> DerefMut for ConfigOrigin<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::CommandLine(value) | Self::ConfigFile(value) | Self::Auto(value) => value,
        }
    }
}

impl<T: Deref> ConfigOrigin<T> {
    pub(crate) fn as_deref(&self) -> ConfigOrigin<&<T as Deref>::Target> {
        self.as_ref().map(|t| t.deref())
    }
}

impl<T> ConfigOrigin<T> {
    /// Construct a new [`ConfigOrigin::CommandLine`] with the given value.
    pub(crate) fn cli(value: T) -> Self {
        Self::CommandLine(value)
    }

    /// Construct a new [`ConfigOrigin::ConfigFile`] with the given value.
    pub(crate) fn config(value: T) -> Self {
        Self::ConfigFile(value)
    }

    /// Construct a new [`ConfigOrigin::Auto`] with the given value.
    pub(crate) fn auto(value: T) -> Self {
        Self::Auto(value)
    }

    /// Consume the [`ConfigOrigin`], mapping the internal value with the
    /// given function, and inserting it into a new [`ConfigOrigin`] of the same
    /// variant.
    pub(crate) fn map<U, F>(self, f: F) -> ConfigOrigin<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            ConfigOrigin::ConfigFile(value) => ConfigOrigin::ConfigFile(f(value)),
            ConfigOrigin::CommandLine(value) => ConfigOrigin::CommandLine(f(value)),
            ConfigOrigin::Auto(value) => ConfigOrigin::Auto(f(value)),
        }
    }

    fn as_ref(&self) -> ConfigOrigin<&T> {
        match *self {
            ConfigOrigin::ConfigFile(ref value) => ConfigOrigin::ConfigFile(value),
            ConfigOrigin::CommandLine(ref value) => ConfigOrigin::CommandLine(value),
            ConfigOrigin::Auto(ref value) => ConfigOrigin::Auto(value),
        }
    }

    /// Determine if this [`Option<ConfigOrigin>`] should be output when serializing.
    /// We only serialize if the value is `Some(ConfigFile)`. All other
    /// [`Option`] and [`ConfigOrigin`] variants are not serialized.
    pub(crate) fn should_skip_serializing_option(origin: &Option<Self>) -> bool {
        !matches!(origin, Some(Self::ConfigFile(_)))
    }
}

impl<T, E> ConfigOrigin<Result<T, E>> {
    /// Swap a ConfigOrigin<Result<T, E>>'s [`ConfigOrigin`] and [`Result`] types,
    /// turning it into a `Result<ConfigOrigin<T>, E>`.
    pub(crate) fn transpose_err(self) -> Result<ConfigOrigin<T>, E> {
        match self {
            ConfigOrigin::ConfigFile(Ok(value)) => Ok(ConfigOrigin::ConfigFile(value)),
            ConfigOrigin::CommandLine(Ok(value)) => Ok(ConfigOrigin::CommandLine(value)),
            ConfigOrigin::Auto(Ok(value)) => Ok(ConfigOrigin::Auto(value)),
            ConfigOrigin::ConfigFile(Err(err))
            | ConfigOrigin::CommandLine(Err(err))
            | ConfigOrigin::Auto(Err(err)) => Err(err),
        }
    }
}
