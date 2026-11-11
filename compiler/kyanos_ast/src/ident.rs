// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Identifier AST representations.

use std::fmt;

use crate::Span;

/// A standard identifier (e.g. `foo`, `_bar`, `$value`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

impl Ident {
    /// Creates a new identifier with name and span.
    pub fn new(name: impl Into<String>, span: Span) -> Self {
        Self {
            name: name.into(),
            span,
        }
    }

    /// Returns the string slice of the identifier.
    pub fn as_str(&self) -> &str {
        &self.name
    }

    /// Returns true if this identifier is the wildcard/discard placeholder `_`.
    pub fn is_underscore(&self) -> bool {
        self.name == "_"
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// A private identifier starting with `#` for class private members (e.g. `#field`, `#method`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PrivateIdent {
    pub name: String,
    pub span: Span,
}

impl PrivateIdent {
    /// Creates a new private identifier with name and span.
    pub fn new(name: impl Into<String>, span: Span) -> Self {
        Self {
            name: name.into(),
            span,
        }
    }

    /// Returns the string slice including `#`.
    pub fn as_str(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for PrivateIdent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}
