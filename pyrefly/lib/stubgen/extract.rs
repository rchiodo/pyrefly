/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Extracts stub declarations from a type-checked module.
//!
//! Walks the module's AST in source order and uses the binding/answer
//! system to resolve types for each declaration.

/// A single module's stub content, in source order.
pub struct ModuleStub {
    pub items: Vec<StubItem>,
    /// Whether any item uses `Incomplete` (so we know whether to
    /// emit `from _typeshed import Incomplete`).
    pub uses_incomplete: bool,
}

pub enum StubItem {
    Import(StubImport),
    Function(StubFunction),
    Class(StubClass),
    Variable(StubVariable),
    TypeAlias(StubTypeAlias),
}

pub struct StubImport {
    pub text: String,
}

pub struct StubFunction {
    pub name: String,
    pub is_async: bool,
    pub decorators: Vec<String>,
    pub params: Vec<StubParam>,
    pub return_type: Option<String>,
    pub docstring: Option<String>,
}

pub struct StubParam {
    pub prefix: &'static str,
    pub name: String,
    pub annotation: Option<String>,
    pub default: Option<String>,
}

pub struct StubClass {
    pub name: String,
    pub bases: String,
    pub decorators: Vec<String>,
    pub body: Vec<StubItem>,
    pub docstring: Option<String>,
}

pub struct StubVariable {
    pub name: String,
    pub annotation: Option<String>,
    pub value: Option<String>,
}

pub struct StubTypeAlias {
    /// e.g. `type Vector = list[float]`.
    pub text: String,
}

/// Configuration for stub extraction.
pub struct ExtractConfig {
    pub include_private: bool,
    pub include_docstrings: bool,
}
