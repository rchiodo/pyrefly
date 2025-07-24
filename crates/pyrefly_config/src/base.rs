/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use clap::ValueEnum;
use serde::Deserialize;
use serde::Serialize;
use toml::Table;

use crate::error::ErrorDisplayConfig;
use crate::module_wildcard::ModuleWildcard;

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone, Copy, Default)]
#[derive(ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum UntypedDefBehavior {
    #[default]
    CheckAndInferReturnType,
    CheckAndInferReturnAny,
    SkipAndInferReturnAny,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "kebab-case")]
pub struct ConfigBase {
    /// Errors to silence (or not) when printing errors.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub errors: Option<ErrorDisplayConfig>,

    /// Consider any ignore (including from other tools) to ignore an error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permissive_ignores: Option<bool>,

    /// Modules from which import errors should be ignored
    /// and the module should always be replaced with `typing.Any`
    #[serde(
        default,
        skip_serializing_if = "crate::util::none_or_empty",
        // TODO(connernilsen): DON'T COPY THIS TO NEW FIELDS. This is a temporary
        // alias while we migrate existing fields from snake case to kebab case.
        alias = "replace_imports_with_any"
    )]
    pub(crate) replace_imports_with_any: Option<Vec<ModuleWildcard>>,

    /// Modules from which import errors should be
    /// ignored. The module is only replaced with `typing.Any` if it can't be found.
    #[serde(default, skip_serializing_if = "crate::util::none_or_empty")]
    pub(crate) ignore_missing_imports: Option<Vec<ModuleWildcard>>,

    /// How should we handle analyzing and inferring the function signature if it's untyped?
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        // TODO(connernilsen): DON'T COPY THIS TO NEW FIELDS. This is a temporary
        // alias while we migrate existing fields from snake case to kebab case.
        alias = "untyped_def_behavior"
    )]
    pub untyped_def_behavior: Option<UntypedDefBehavior>,

    /// Whether to ignore type errors in generated code. By default this is disabled.
    /// Generated code is defined as code that contains the marker string `@` immediately followed by `generated`.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        // TODO(connernilsen): DON'T COPY THIS TO NEW FIELDS. This is a temporary
        // alias while we migrate existing fields from snake case to kebab case.
        alias = "ignore_errors_in_generated_code"
    )]
    pub ignore_errors_in_generated_code: Option<bool>,

    /// Any unknown config items
    #[serde(default, flatten)]
    pub(crate) extras: ExtraConfigs,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(transparent)]
pub(crate) struct ExtraConfigs(pub(crate) Table);

// `Value` types in `Table` might not be `Eq`, but we don't actually care about that w.r.t. `ConfigFile`
impl Eq for ExtraConfigs {}

impl PartialEq for ExtraConfigs {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl ConfigBase {
    pub fn get_errors(base: &Self) -> Option<&ErrorDisplayConfig> {
        base.errors.as_ref()
    }

    pub(crate) fn get_replace_imports_with_any(base: &Self) -> Option<&[ModuleWildcard]> {
        base.replace_imports_with_any.as_deref()
    }

    pub(crate) fn get_ignore_missing_imports(base: &Self) -> Option<&[ModuleWildcard]> {
        base.ignore_missing_imports.as_deref()
    }

    pub fn get_untyped_def_behavior(base: &Self) -> Option<UntypedDefBehavior> {
        base.untyped_def_behavior
    }

    pub fn get_ignore_errors_in_generated_code(base: &Self) -> Option<bool> {
        base.ignore_errors_in_generated_code
    }
}
