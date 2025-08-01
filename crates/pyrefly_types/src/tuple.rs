/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use std::fmt;
use std::fmt::Display;

use pyrefly_derive::TypeEq;
use pyrefly_derive::Visit;
use pyrefly_derive::VisitMut;
use pyrefly_util::display::commas_iter;

use crate::types::Type;

/*
Eventually this will have to be generalized enough to handle at least four cases:

1. the gradually-typed tuple tuple[Any, ...]
2. normal tuples as are handled here
3. variadic tuples with a splatted typevartuple variable
4. indefinite-length tuples tuple[int, ...] (whose length is supposed to be treated soundly, not gradually, IIRC)
*/

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Visit, VisitMut, TypeEq)]
pub enum Tuple {
    // tuple[t1, t2]
    Concrete(Vec<Type>),
    // tuple[t1, ...]
    Unbounded(Box<Type>),
    // tuple[t1, t2, *t3, t4, t5], where t3 must be a type var tuple or unbounded tuple
    Unpacked(Box<(Vec<Type>, Type, Vec<Type>)>),
}

impl Default for Tuple {
    fn default() -> Self {
        Self::Concrete(Vec::new())
    }
}

impl Tuple {
    // Check if this is tuple[Any, ...]
    pub fn is_any_tuple(&self) -> bool {
        match self {
            Self::Unbounded(ty) => ty.is_any(),
            _ => false,
        }
    }

    pub fn concrete(elts: Vec<Type>) -> Self {
        Self::Concrete(elts)
    }

    pub fn unbounded(elt: Type) -> Self {
        Self::Unbounded(Box::new(elt))
    }

    pub fn unpacked(prefix: Vec<Type>, middle: Type, suffix: Vec<Type>) -> Tuple {
        if prefix.is_empty()
            && suffix.is_empty()
            && let Type::Tuple(tuple) = middle
        {
            return tuple;
        }
        Self::Unpacked(Box::new((prefix, middle, suffix)))
    }

    pub fn fmt_with_type<'a, D: Display + 'a>(
        &'a self,
        f: &mut fmt::Formatter<'_>,
        wrap: impl Fn(&'a Type) -> D,
    ) -> fmt::Result {
        let contents = match self {
            Self::Concrete(elts) => {
                if elts.is_empty() {
                    "()".to_owned()
                } else {
                    format!("{}", commas_iter(|| elts.iter().map(&wrap)))
                }
            }
            Self::Unbounded(ty) => format!("{}, ...", wrap(ty)),
            Self::Unpacked(box (prefix, unpacked, suffix)) => {
                let prefix = if prefix.is_empty() {
                    "".to_owned()
                } else {
                    format!("{}, ", commas_iter(|| prefix.iter().map(&wrap)))
                };
                let suffix = if suffix.is_empty() {
                    "".to_owned()
                } else {
                    format!(", {}", commas_iter(|| suffix.iter().map(&wrap)))
                };
                let unpacked = format!("*{}", wrap(unpacked));
                format!("{prefix}{unpacked}{suffix}")
            }
        };
        write!(f, "tuple[{contents}]")
    }
}
