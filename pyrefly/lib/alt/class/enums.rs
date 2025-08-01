/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use std::sync::Arc;

use ruff_python_ast::name::Name;
use starlark_map::small_set::SmallSet;

use crate::alt::answers::LookupAnswer;
use crate::alt::answers_solver::AnswersSolver;
use crate::alt::class::class_field::ClassFieldInitialization;
use crate::types::class::Class;
use crate::types::literal::Lit;
use crate::types::types::Type;

impl<'a, Ans: LookupAnswer> AnswersSolver<'a, Ans> {
    pub fn get_enum_member(&self, cls: &Class, name: &Name) -> Option<Lit> {
        self.get_field_from_current_class_only(cls, name, false)
            .and_then(|field| Arc::unwrap_or_clone(field).as_enum_member(cls))
    }

    pub fn get_enum_members(&self, cls: &Class) -> SmallSet<Lit> {
        cls.fields()
            .filter_map(|f| self.get_enum_member(cls, f))
            .collect()
    }

    pub fn is_valid_enum_member(
        &self,
        name: &Name,
        ty: &Type,
        initialization: &ClassFieldInitialization,
    ) -> bool {
        // Names starting but not ending with __ are private
        // Names starting and ending with _ are reserved by the enum
        if name.starts_with("__") && !name.ends_with("__")
            || name.starts_with("_") && name.ends_with("_")
        {
            return false;
        }
        // Enum members must be initialized on the class
        if !matches!(*initialization, ClassFieldInitialization::ClassBody(_)) {
            return false;
        }
        match ty {
            // Methods decorated with @member are members
            _ if ty.has_enum_member_decoration() => true,
            // Callables are not valid enum members
            Type::BoundMethod(_) | Type::Callable(_) | Type::Function(_) => false,
            // Values initialized with nonmember() are not members
            Type::ClassType(cls)
                if cls.has_qname("enum", "nonmember")
                    || cls.is_builtin("staticmethod")
                    || cls.is_builtin("classmethod")
                    || cls.has_qname("types", "DynamicClassAttribute")
                    || cls.has_qname("enum", "property") =>
            {
                false
            }
            _ => true,
        }
    }
}
