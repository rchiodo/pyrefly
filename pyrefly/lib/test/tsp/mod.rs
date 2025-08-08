/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Tests for TSP (Type Server Protocol) request handlers

pub mod get_builtin_type;
pub mod get_function_parts;
pub mod get_matching_overloads;
pub mod get_overloads;
pub mod get_repr;
pub mod get_symbol;
pub mod get_symbols_for_file;
pub mod get_type;
pub mod get_type_alias_info;
pub mod resolve_import_declaration;
pub mod search_for_type_attribute;
pub mod tsp_interaction;
pub mod util;
