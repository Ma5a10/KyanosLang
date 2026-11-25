// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Declaration, attribute, C-FFI, module, and program parser routines for Kyanos.

use kyanos_ast::{
    Accessibility, AccessorKind, Attribute, AttributeArg, AttributeInput, ClassDecl,
    ClassElementName, ClassField, ClassMember, ClassMethod, ConstructorDecl, Decl, EnumDecl,
    EnumPayloadField, EnumVariant, ExportSpecifier, ExportStmt, ExternBlockDecl,
    ExternFunctionDecl, ExternMember, ExternParameterList, ExternVariableDecl, FunctionDecl,
    GetterSetterDecl, Ident, ImportClause, ImportSpecifier, ImportStmt, IndexSignature,
    InterfaceDecl, InterfaceMember, InterfaceMethod, InterfaceProperty, LiteralKind, MethodDecl,
    Parameter, ParameterList, PredefinedType, PrivateIdent, Program, RestParameter,
    SingleExternDecl, SourceElement, Span, StringLiteral, StructDecl, StructField, StructMember,
    TypeAliasDecl, TypeReference,
};
use kyanos_lexer::TokenKind;

use crate::error::{ParseError, ParseResult};
use crate::types::token_to_predefined_type;
use crate::Parser;

impl<'a> Parser<'a> {
    /// Parses an entire Kyanos source file into a [`Program`] AST.
    pub fn parse_program_inner(&mut self) -> Program {
        let mut elements = Vec::new();
        let mut program_span = Span::DUMMY;

        while !self.is_eof() {
            if self.at(TokenKind::CloseBrace) {
                let tok = self.bump();
                self.report_error(ParseError::unexpected(
                    "top-level declaration or statement",
                    tok.kind,
                    tok.span,
                ));
                continue;
            }

            let start_cursor = self.checkpoint().cursor;
            match self.parse_source_element() {
                Ok(elem) => {
                    program_span = program_span.merge(elem.span());
                    elements.push(elem);
                }
                Err(err) => {
                    self.report_error(err);
                    self.synchronize_from(start_cursor);
                }
            }
        }

        if elements.is_empty() {
            Program::empty(Span::new(0, 0))
        } else {
            Program::new(elements, program_span)
        }
    }

    /// Parses a single top-level `SourceElement`.
    pub fn parse_source_element(&mut self) -> ParseResult<SourceElement> {
        if self.at(TokenKind::Import) {
            return Ok(SourceElement::Import(self.parse_import_stmt()?));
        }

        let attrs = if self.at(TokenKind::HashBracket) {
            self.parse_attributes()?
        } else {
            Vec::new()
        };

        if self.at(TokenKind::Export) {
            return Ok(SourceElement::Export(self.parse_export_stmt(attrs)?));
        }

        if !attrs.is_empty() || self.at_top_level_decl_start() {
            return Ok(SourceElement::Decl(self.parse_decl_with_attributes(attrs)?));
        }

        Ok(SourceElement::Stmt(self.parse_stmt()?))
    }

    #[inline]
    fn at_top_level_decl_start(&self) -> bool {
        matches!(
            self.current_kind(),
            TokenKind::Let | TokenKind::Const | TokenKind::Using
        ) || self.at_declaration_start()
    }

    /// Returns `true` if the current token begins a non-variable declaration
    /// (`#[...]`, `function`, `async function`, `struct`, `class`, `interface`,
    /// `enum`, `type`, or `extern`).
    #[inline]
    pub(crate) fn at_declaration_start(&self) -> bool {
        match self.current_kind() {
            TokenKind::HashBracket
            | TokenKind::Function
            | TokenKind::Struct
            | TokenKind::Class
            | TokenKind::Interface
            | TokenKind::Enum
            | TokenKind::Type
            | TokenKind::Extern => true,
            TokenKind::Async => self.peek_kind(1) == TokenKind::Function,
            _ => false,
        }
    }

    /// Parses a declaration (including optional leading `#[...]` attributes).
    pub fn parse_decl(&mut self) -> ParseResult<Decl> {
        let attrs = if self.at(TokenKind::HashBracket) {
            self.parse_attributes()?
        } else {
            Vec::new()
        };
        self.parse_decl_with_attributes(attrs)
    }

    fn parse_decl_with_attributes(&mut self, attrs: Vec<Attribute>) -> ParseResult<Decl> {
        let start = attrs
            .first()
            .map(|a| a.span)
            .unwrap_or_else(|| self.current_span());

        match self.current_kind() {
            TokenKind::Let | TokenKind::Const => {
                if !attrs.is_empty() {
                    self.report_error(ParseError::syntax(
                        "Attributes cannot be attached to variable declarations",
                        start,
                    ));
                }
                let var_stmt = self.parse_variable_stmt_inner()?;
                let id = self.alloc_id();
                Ok(Decl::variable(var_stmt).with_id(id))
            }
            TokenKind::Using => {
                if !attrs.is_empty() {
                    self.report_error(ParseError::syntax(
                        "Attributes cannot be attached to 'using' declarations",
                        start,
                    ));
                }
                let using_stmt = self.parse_using_stmt_inner()?;
                let id = self.alloc_id();
                Ok(Decl::using_decl(using_stmt).with_id(id))
            }
            TokenKind::Async | TokenKind::Function => self.parse_function_decl(attrs, start),
            TokenKind::Struct => self.parse_struct_decl(attrs, start),
            TokenKind::Class => self.parse_class_decl(attrs, start),
            TokenKind::Interface => {
                if !attrs.is_empty() {
                    self.report_error(ParseError::syntax(
                        "Attributes cannot be attached to interface declarations",
                        start,
                    ));
                }
                self.parse_interface_decl()
            }
            TokenKind::Enum => self.parse_enum_decl(attrs, start),
            TokenKind::Type => self.parse_type_alias_decl(attrs, start),
            TokenKind::Extern => self.parse_extern_decl(attrs, start),
            other => Err(ParseError::unexpected(
                "declaration",
                other,
                self.current_span(),
            )),
        }
    }

    // -------------------------------------------------------------------------
    // Module Statements (`import` / `export`)
    // -------------------------------------------------------------------------

    fn parse_import_stmt(&mut self) -> ParseResult<ImportStmt> {
        let import_tok = self.expect(TokenKind::Import, "'import'")?;

        if self.at(TokenKind::StringLiteral) {
            let source = self.parse_string_literal_node()?;
            self.expect_eos_or_recover();
            let span = self.span_from(import_tok.span);
            return Ok(ImportStmt::side_effect(source, span));
        }

        let clause = if self.at(TokenKind::OpenBrace) {
            ImportClause::Named(self.parse_import_named_clause()?)
        } else if self.eat(TokenKind::Star).is_some() {
            self.expect(TokenKind::As, "'as'")?;
            let alias_tok = self.expect(TokenKind::Identifier, "namespace identifier")?;
            let alias = Ident::new(self.slice(alias_tok.span), alias_tok.span);
            ImportClause::Namespace(alias)
        } else if self.at(TokenKind::Identifier) {
            let default_tok = self.bump();
            let default_name = Ident::new(self.slice(default_tok.span), default_tok.span);
            let named = if self.eat(TokenKind::Comma).is_some() {
                self.parse_import_named_clause()?
            } else {
                Vec::new()
            };
            ImportClause::Default {
                default_name,
                named,
            }
        } else {
            let tok = self.current();
            return Err(ParseError::unexpected("import clause", tok.kind, tok.span));
        };

        self.expect(TokenKind::From, "'from'")?;
        let source = self.parse_string_literal_node()?;
        self.expect_eos_or_recover();
        let span = self.span_from(import_tok.span);
        Ok(ImportStmt::new(Some(clause), source, span))
    }

    fn parse_import_named_clause(&mut self) -> ParseResult<Vec<ImportSpecifier>> {
        self.expect(TokenKind::OpenBrace, "'{'")?;
        let mut specs = Vec::new();
        while !self.at(TokenKind::CloseBrace) && !self.is_eof() {
            let name_tok = self.expect(TokenKind::Identifier, "import specifier")?;
            let name = Ident::new(self.slice(name_tok.span), name_tok.span);
            let alias = if self.eat(TokenKind::As).is_some() {
                let alias_tok = self.expect(TokenKind::Identifier, "alias identifier")?;
                Some(Ident::new(self.slice(alias_tok.span), alias_tok.span))
            } else {
                None
            };
            let span = self.span_from(name_tok.span);
            specs.push(ImportSpecifier::new(name, alias, span));
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        self.expect(TokenKind::CloseBrace, "'}'")?;
        Ok(specs)
    }

    fn parse_export_stmt(&mut self, attrs: Vec<Attribute>) -> ParseResult<ExportStmt> {
        let start = attrs
            .first()
            .map(|a| a.span)
            .unwrap_or_else(|| self.current_span());
        self.expect(TokenKind::Export, "'export'")?;

        if self.eat(TokenKind::Default).is_some() {
            if !attrs.is_empty() {
                self.report_error(ParseError::syntax(
                    "Attributes cannot be attached to 'export default'",
                    start,
                ));
            }
            let expr = self.parse_expr()?;
            self.expect_eos_or_recover();
            let span = self.span_from(start);
            return Ok(ExportStmt::default_expr(expr, span));
        }

        if self.at(TokenKind::OpenBrace) {
            if !attrs.is_empty() {
                self.report_error(ParseError::syntax(
                    "Attributes cannot be attached to named export clauses",
                    start,
                ));
            }
            let specifiers = self.parse_export_clause()?;
            let source = if self.eat(TokenKind::From).is_some() {
                Some(self.parse_string_literal_node()?)
            } else {
                None
            };
            self.expect_eos_or_recover();
            let span = self.span_from(start);
            return Ok(ExportStmt::named(specifiers, source, span));
        }

        if self.at(TokenKind::HashBracket) {
            self.report_error(
                ParseError::syntax(
                    "Attributes should appear before 'export'",
                    self.current_span(),
                )
                .with_hint("Move '#[...]' before the 'export' keyword."),
            );
        }

        let decl = self.parse_decl()?;
        let span = self.span_from(start);
        Ok(ExportStmt::decl(attrs, decl, span))
    }

    fn parse_export_clause(&mut self) -> ParseResult<Vec<ExportSpecifier>> {
        self.expect(TokenKind::OpenBrace, "'{'")?;
        let mut specs = Vec::new();
        while !self.at(TokenKind::CloseBrace) && !self.is_eof() {
            let name_tok = self.expect(TokenKind::Identifier, "export specifier")?;
            let name = Ident::new(self.slice(name_tok.span), name_tok.span);
            let alias = if self.eat(TokenKind::As).is_some() {
                let alias_tok = self.expect(TokenKind::Identifier, "alias identifier")?;
                Some(Ident::new(self.slice(alias_tok.span), alias_tok.span))
            } else {
                None
            };
            let span = self.span_from(name_tok.span);
            specs.push(ExportSpecifier::new(name, alias, span));
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        self.expect(TokenKind::CloseBrace, "'}'")?;
        Ok(specs)
    }

    // -------------------------------------------------------------------------
    // Attributes (`#[...]`)
    // -------------------------------------------------------------------------

    /// Parses zero or more `#[...]` attribute annotations.
    pub fn parse_attributes(&mut self) -> ParseResult<Vec<Attribute>> {
        let mut attrs = Vec::new();
        while self.at(TokenKind::HashBracket) {
            attrs.push(self.parse_attribute()?);
        }
        Ok(attrs)
    }

    fn parse_attribute(&mut self) -> ParseResult<Attribute> {
        let open = self.expect(TokenKind::HashBracket, "'#['")?;
        let name_tok = self.expect(TokenKind::Identifier, "attribute name")?;
        let name = Ident::new(self.slice(name_tok.span), name_tok.span);

        let input = if self.eat(TokenKind::OpenParen).is_some() {
            let args = self.with_grouping(|p| {
                let mut list = Vec::new();
                while !p.at(TokenKind::CloseParen) && !p.is_eof() {
                    let arg_start = p.current_span();
                    let arg_name =
                        if p.at(TokenKind::Identifier) && p.peek_kind(1) == TokenKind::Assign {
                            let key_tok = p.bump();
                            let key = Ident::new(p.slice(key_tok.span), key_tok.span);
                            p.bump(); // consume '='
                            Some(key)
                        } else {
                            None
                        };
                    let value = p.parse_expr()?;
                    let arg_span = p.span_from(arg_start);
                    list.push(AttributeArg::new(arg_name, value, arg_span));
                    if p.eat(TokenKind::Comma).is_none() {
                        break;
                    }
                }
                Ok::<_, ParseError>(list)
            })?;
            self.expect(TokenKind::CloseParen, "')'")?;
            AttributeInput::Args(args)
        } else if self.eat(TokenKind::Assign).is_some() {
            AttributeInput::Value(self.with_grouping(|p| p.parse_expr())?)
        } else {
            AttributeInput::None
        };

        self.expect(TokenKind::CloseBracket, "']'")?;
        let span = self.span_from(open.span);
        Ok(Attribute { name, input, span })
    }

    // -------------------------------------------------------------------------
    // Function, Struct, Class, Interface, Enum, Type Alias
    // -------------------------------------------------------------------------

    fn parse_function_decl(
        &mut self,
        attributes: Vec<Attribute>,
        start: Span,
    ) -> ParseResult<Decl> {
        let is_async = self.eat(TokenKind::Async).is_some();
        self.expect(TokenKind::Function, "'function'")?;
        let name_tok = self.expect(TokenKind::Identifier, "function name")?;
        let name = Ident::new(self.slice(name_tok.span), name_tok.span);

        let type_parameters = if self.at(TokenKind::LessThan) {
            Some(self.parse_type_parameters()?)
        } else {
            None
        };
        let params = self.parse_parameter_list()?;
        let return_type = if self.eat(TokenKind::Colon).is_some() {
            Some(self.parse_return_type()?)
        } else {
            None
        };
        let body = if self.at(TokenKind::OpenBrace) {
            Some(self.parse_block()?)
        } else {
            self.expect_eos_or_recover();
            None
        };

        let span = self.span_from(start);
        let id = self.alloc_id();
        Ok(Decl::function(FunctionDecl::new(
            attributes,
            is_async,
            name,
            type_parameters,
            params,
            return_type,
            body,
            span,
        ))
        .with_id(id))
    }

    fn parse_struct_decl(&mut self, attributes: Vec<Attribute>, start: Span) -> ParseResult<Decl> {
        self.expect(TokenKind::Struct, "'struct'")?;
        let name_tok = self.expect(TokenKind::Identifier, "struct name")?;
        let name = Ident::new(self.slice(name_tok.span), name_tok.span);
        let type_parameters = if self.at(TokenKind::LessThan) {
            Some(self.parse_type_parameters()?)
        } else {
            None
        };

        self.expect(TokenKind::OpenBrace, "'{'")?;
        let mut members = Vec::new();
        while !self.at(TokenKind::CloseBrace) && !self.is_eof() {
            if self.eat(TokenKind::Semicolon).is_some() {
                continue;
            }
            let start_cursor = self.checkpoint().cursor;
            match self.parse_struct_member() {
                Ok(member) => members.push(member),
                Err(err) => {
                    self.report_error(err);
                    self.synchronize_from(start_cursor);
                }
            }
        }
        self.expect(TokenKind::CloseBrace, "'}'")?;

        let span = self.span_from(start);
        let id = self.alloc_id();
        Ok(Decl::struct_decl(StructDecl::new(
            attributes,
            name,
            type_parameters,
            members,
            span,
        ))
        .with_id(id))
    }

    fn parse_struct_member(&mut self) -> ParseResult<StructMember> {
        let start = self.current_span();
        let accessibility = self.parse_optional_accessibility();

        if self.eat(TokenKind::Constructor).is_some() {
            let params = self.parse_parameter_list()?;
            let body = self.parse_block()?;
            let span = self.span_from(start);
            return Ok(StructMember::Constructor(ConstructorDecl {
                accessibility,
                params,
                body,
                span,
            }));
        }

        let is_static = self.eat(TokenKind::Static).is_some();

        if self.eat(TokenKind::Readonly).is_some() {
            let name_tok = self.expect(TokenKind::Identifier, "field identifier")?;
            let name = Ident::new(self.slice(name_tok.span), name_tok.span);
            return self.finish_struct_field(start, accessibility, is_static, true, name);
        }

        if self.eat(TokenKind::Async).is_some() {
            let name_tok = self.expect(TokenKind::Identifier, "method identifier")?;
            let name = Ident::new(self.slice(name_tok.span), name_tok.span);
            return self.finish_struct_method(start, accessibility, is_static, true, name);
        }

        let name_tok = self.expect(TokenKind::Identifier, "struct member name")?;
        let name = Ident::new(self.slice(name_tok.span), name_tok.span);

        if self.at(TokenKind::LessThan) || self.at(TokenKind::OpenParen) {
            self.finish_struct_method(start, accessibility, is_static, false, name)
        } else {
            self.finish_struct_field(start, accessibility, is_static, false, name)
        }
    }

    fn finish_struct_field(
        &mut self,
        start: Span,
        accessibility: Option<Accessibility>,
        is_static: bool,
        is_readonly: bool,
        name: Ident,
    ) -> ParseResult<StructMember> {
        let ty = if self.eat(TokenKind::Colon).is_some() {
            Some(self.parse_type()?)
        } else {
            None
        };
        let init = if self.eat(TokenKind::Assign).is_some() {
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.expect_eos_or_recover();
        let span = self.span_from(start);
        Ok(StructMember::Field(StructField {
            accessibility,
            is_static,
            is_readonly,
            name,
            ty,
            init,
            span,
        }))
    }

    fn finish_struct_method(
        &mut self,
        start: Span,
        accessibility: Option<Accessibility>,
        is_static: bool,
        is_async: bool,
        name: Ident,
    ) -> ParseResult<StructMember> {
        let type_parameters = if self.at(TokenKind::LessThan) {
            Some(self.parse_type_parameters()?)
        } else {
            None
        };
        let params = self.parse_parameter_list()?;
        let return_type = if self.eat(TokenKind::Colon).is_some() {
            Some(self.parse_return_type()?)
        } else {
            None
        };
        let body = self.parse_block()?;
        let span = self.span_from(start);
        Ok(StructMember::Method(MethodDecl {
            accessibility,
            is_static,
            is_async,
            name,
            type_parameters,
            params,
            return_type,
            body,
            span,
        }))
    }

    fn parse_class_decl(&mut self, attributes: Vec<Attribute>, start: Span) -> ParseResult<Decl> {
        self.expect(TokenKind::Class, "'class'")?;
        let name = self.parse_class_element_name()?;
        let type_parameters = if self.at(TokenKind::LessThan) {
            Some(self.parse_type_parameters()?)
        } else {
            None
        };

        let extends = if self.eat(TokenKind::Extends).is_some() {
            Some(self.parse_type_reference()?)
        } else {
            None
        };

        let implements = if self.eat(TokenKind::Implements).is_some() {
            self.parse_type_reference_list()?
        } else {
            Vec::new()
        };

        self.expect(TokenKind::OpenBrace, "'{'")?;
        let mut members = Vec::new();
        while !self.at(TokenKind::CloseBrace) && !self.is_eof() {
            if self.eat(TokenKind::Semicolon).is_some() {
                continue;
            }
            let start_cursor = self.checkpoint().cursor;
            match self.parse_class_member() {
                Ok(member) => members.push(member),
                Err(err) => {
                    self.report_error(err);
                    self.synchronize_from(start_cursor);
                }
            }
        }
        self.expect(TokenKind::CloseBrace, "'}'")?;

        let span = self.span_from(start);
        let id = self.alloc_id();
        Ok(Decl::class_decl(ClassDecl::new(
            attributes,
            name,
            type_parameters,
            extends,
            implements,
            members,
            span,
        ))
        .with_id(id))
    }

    fn parse_class_member(&mut self) -> ParseResult<ClassMember> {
        let start = self.current_span();
        let accessibility = self.parse_optional_accessibility();

        if self.eat(TokenKind::Constructor).is_some() {
            let params = self.parse_parameter_list()?;
            let body = self.parse_block()?;
            let span = self.span_from(start);
            return Ok(ClassMember::Constructor(ConstructorDecl {
                accessibility,
                params,
                body,
                span,
            }));
        }

        let is_static = self.eat(TokenKind::Static).is_some();

        if self.eat(TokenKind::Readonly).is_some() {
            let name = self.parse_class_element_name()?;
            return self.finish_class_field(start, accessibility, is_static, true, name);
        }

        if self.eat(TokenKind::Async).is_some() {
            let name = self.parse_class_element_name()?;
            return self.finish_class_method(start, accessibility, is_static, true, name);
        }

        // Contextual `get` / `set` accessor when followed on the same line by an identifier or `#private`.
        let next = self.peek(1);
        if (self.at_ident_str("get") || self.at_ident_str("set"))
            && !next.has_leading_newline
            && matches!(
                next.kind,
                TokenKind::Identifier | TokenKind::PrivateIdentifier
            )
        {
            let kind = if self.at_ident_str("get") {
                AccessorKind::Get
            } else {
                AccessorKind::Set
            };
            self.bump(); // consume `get` or `set`
            let name = self.parse_class_element_name()?;
            let params = self.parse_parameter_list()?;
            let return_type = if self.eat(TokenKind::Colon).is_some() {
                Some(self.parse_type()?)
            } else {
                None
            };
            let body = self.parse_block()?;
            let span = self.span_from(start);
            return Ok(ClassMember::GetterSetter(GetterSetterDecl {
                accessibility,
                is_static,
                kind,
                name,
                params,
                return_type,
                body,
                span,
            }));
        }

        let name = self.parse_class_element_name()?;
        if self.at(TokenKind::LessThan) || self.at(TokenKind::OpenParen) {
            self.finish_class_method(start, accessibility, is_static, false, name)
        } else {
            self.finish_class_field(start, accessibility, is_static, false, name)
        }
    }

    fn finish_class_field(
        &mut self,
        start: Span,
        accessibility: Option<Accessibility>,
        is_static: bool,
        is_readonly: bool,
        name: ClassElementName,
    ) -> ParseResult<ClassMember> {
        let ty = if self.eat(TokenKind::Colon).is_some() {
            Some(self.parse_type()?)
        } else {
            None
        };
        let init = if self.eat(TokenKind::Assign).is_some() {
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.expect_eos_or_recover();
        let span = self.span_from(start);
        Ok(ClassMember::Field(ClassField {
            accessibility,
            is_static,
            is_readonly,
            name,
            ty,
            init,
            span,
        }))
    }

    fn finish_class_method(
        &mut self,
        start: Span,
        accessibility: Option<Accessibility>,
        is_static: bool,
        is_async: bool,
        name: ClassElementName,
    ) -> ParseResult<ClassMember> {
        let type_parameters = if self.at(TokenKind::LessThan) {
            Some(self.parse_type_parameters()?)
        } else {
            None
        };
        let params = self.parse_parameter_list()?;
        let return_type = if self.eat(TokenKind::Colon).is_some() {
            Some(self.parse_return_type()?)
        } else {
            None
        };
        let body = self.parse_block()?;
        let span = self.span_from(start);
        Ok(ClassMember::Method(ClassMethod {
            accessibility,
            is_static,
            is_async,
            name,
            type_parameters,
            params,
            return_type,
            body,
            span,
        }))
    }

    fn parse_class_element_name(&mut self) -> ParseResult<ClassElementName> {
        let tok = self.current();
        match tok.kind {
            TokenKind::Identifier => {
                self.bump();
                Ok(ClassElementName::Ident(Ident::new(
                    self.slice(tok.span),
                    tok.span,
                )))
            }
            TokenKind::PrivateIdentifier => {
                self.bump();
                Ok(ClassElementName::Private(PrivateIdent::new(
                    self.slice(tok.span),
                    tok.span,
                )))
            }
            other => Err(ParseError::unexpected(
                "identifier or '#private'",
                other,
                tok.span,
            )),
        }
    }

    fn parse_interface_decl(&mut self) -> ParseResult<Decl> {
        let iface_tok = self.expect(TokenKind::Interface, "'interface'")?;
        let name_tok = self.expect(TokenKind::Identifier, "interface name")?;
        let name = Ident::new(self.slice(name_tok.span), name_tok.span);
        let type_parameters = if self.at(TokenKind::LessThan) {
            Some(self.parse_type_parameters()?)
        } else {
            None
        };

        let extends = if self.eat(TokenKind::Extends).is_some() {
            self.parse_type_reference_list()?
        } else {
            Vec::new()
        };

        self.expect(TokenKind::OpenBrace, "'{'")?;
        let mut members = Vec::new();
        while !self.at(TokenKind::CloseBrace) && !self.is_eof() {
            if self.eat(TokenKind::Semicolon).is_some() {
                continue;
            }
            let start_cursor = self.checkpoint().cursor;
            match self.parse_interface_member() {
                Ok(member) => members.push(member),
                Err(err) => {
                    self.report_error(err);
                    self.synchronize_from(start_cursor);
                }
            }
        }
        self.expect(TokenKind::CloseBrace, "'}'")?;

        let span = self.span_from(iface_tok.span);
        let id = self.alloc_id();
        Ok(Decl::interface_decl(InterfaceDecl::new(
            name,
            type_parameters,
            extends,
            members,
            span,
        ))
        .with_id(id))
    }

    fn parse_interface_member(&mut self) -> ParseResult<InterfaceMember> {
        let start = self.current_span();

        if self.eat(TokenKind::OpenBracket).is_some() {
            let key_tok = self.expect(TokenKind::Identifier, "index key identifier")?;
            let name = Ident::new(self.slice(key_tok.span), key_tok.span);
            self.expect(TokenKind::Colon, "':'")?;
            let key_tok_ty = self.current();
            let key_type = match key_tok_ty.kind {
                TokenKind::TypeInt => {
                    self.bump();
                    PredefinedType::Int
                }
                TokenKind::TypeString => {
                    self.bump();
                    PredefinedType::String
                }
                other => {
                    return Err(ParseError::unexpected(
                        "'int' or 'string'",
                        other,
                        key_tok_ty.span,
                    ))
                }
            };
            self.expect(TokenKind::CloseBracket, "']'")?;
            self.expect(TokenKind::Colon, "':'")?;
            let value_type = self.parse_type()?;
            self.expect_eos_or_recover();
            let span = self.span_from(start);
            return Ok(InterfaceMember::Index(IndexSignature {
                name,
                key_type,
                value_type,
                span,
            }));
        }

        let is_readonly = self.eat(TokenKind::Readonly).is_some();
        let name_tok = self.expect(TokenKind::Identifier, "interface member name")?;
        let name = Ident::new(self.slice(name_tok.span), name_tok.span);
        let is_optional = self.eat(TokenKind::Question).is_some();

        if !is_readonly && (self.at(TokenKind::LessThan) || self.at(TokenKind::OpenParen)) {
            let type_parameters = if self.at(TokenKind::LessThan) {
                Some(self.parse_type_parameters()?)
            } else {
                None
            };
            let params = self.parse_parameter_list()?;
            self.expect(TokenKind::Colon, "':'")?;
            let return_type = self.parse_return_type()?;
            self.expect_eos_or_recover();
            let span = self.span_from(start);
            Ok(InterfaceMember::Method(InterfaceMethod {
                name,
                is_optional,
                type_parameters,
                params,
                return_type,
                span,
            }))
        } else {
            self.expect(TokenKind::Colon, "':'")?;
            let ty = self.parse_type()?;
            self.expect_eos_or_recover();
            let span = self.span_from(start);
            Ok(InterfaceMember::Property(InterfaceProperty {
                is_readonly,
                name,
                is_optional,
                ty,
                span,
            }))
        }
    }

    fn parse_enum_decl(&mut self, attributes: Vec<Attribute>, start: Span) -> ParseResult<Decl> {
        self.expect(TokenKind::Enum, "'enum'")?;
        let name_tok = self.expect(TokenKind::Identifier, "enum name")?;
        let name = Ident::new(self.slice(name_tok.span), name_tok.span);
        let type_parameters = if self.at(TokenKind::LessThan) {
            Some(self.parse_type_parameters()?)
        } else {
            None
        };

        let tag_type = if self.eat(TokenKind::Colon).is_some() {
            let ty_tok = self.current();
            let pt = token_to_predefined_type(ty_tok.kind).ok_or_else(|| {
                ParseError::unexpected("predefined primitive type", ty_tok.kind, ty_tok.span)
            })?;
            self.bump();
            Some(pt)
        } else {
            None
        };

        self.expect(TokenKind::OpenBrace, "'{'")?;
        let mut variants = Vec::new();
        while !self.at(TokenKind::CloseBrace) && !self.is_eof() {
            let v_tok = self.expect(TokenKind::Identifier, "enum variant name")?;
            let v_name = Ident::new(self.slice(v_tok.span), v_tok.span);

            let variant = if self.eat(TokenKind::OpenParen).is_some() {
                let payload = self.parse_enum_payload_list()?;
                self.expect(TokenKind::CloseParen, "')'")?;
                let v_span = self.span_from(v_tok.span);
                EnumVariant::Tuple {
                    name: v_name,
                    payload,
                    span: v_span,
                }
            } else {
                let value = if self.eat(TokenKind::Assign).is_some() {
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                let v_span = self.span_from(v_tok.span);
                EnumVariant::Value {
                    name: v_name,
                    value,
                    span: v_span,
                }
            };
            variants.push(variant);

            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        self.expect(TokenKind::CloseBrace, "'}'")?;

        let span = self.span_from(start);
        let id = self.alloc_id();
        Ok(Decl::enum_decl(EnumDecl::new(
            attributes,
            name,
            type_parameters,
            tag_type,
            variants,
            span,
        ))
        .with_id(id))
    }

    fn parse_enum_payload_list(&mut self) -> ParseResult<Vec<EnumPayloadField>> {
        self.with_grouping(|p| {
            let mut fields = Vec::new();
            while !p.at(TokenKind::CloseParen) && !p.is_eof() {
                let field_start = p.current_span();
                if p.at(TokenKind::Identifier) && p.peek_kind(1) == TokenKind::Colon {
                    let name_tok = p.bump();
                    let name = Ident::new(p.slice(name_tok.span), name_tok.span);
                    p.bump(); // consume ':'
                    let ty = p.parse_type()?;
                    let span = p.span_from(field_start);
                    fields.push(EnumPayloadField::named(name, ty, span));
                } else {
                    let ty = p.parse_type()?;
                    fields.push(EnumPayloadField::unnamed(ty));
                }

                if p.eat(TokenKind::Comma).is_none() {
                    break;
                }
            }
            Ok(fields)
        })
    }

    fn parse_type_alias_decl(
        &mut self,
        attributes: Vec<Attribute>,
        start: Span,
    ) -> ParseResult<Decl> {
        self.expect(TokenKind::Type, "'type'")?;
        let name_tok = self.expect(TokenKind::Identifier, "type alias name")?;
        let name = Ident::new(self.slice(name_tok.span), name_tok.span);
        let type_parameters = if self.at(TokenKind::LessThan) {
            Some(self.parse_type_parameters()?)
        } else {
            None
        };
        self.expect(TokenKind::Assign, "'='")?;
        let ty = self.parse_type()?;
        self.expect_eos_or_recover();
        let span = self.span_from(start);
        let id = self.alloc_id();
        Ok(Decl::type_alias(TypeAliasDecl::new(
            attributes,
            name,
            type_parameters,
            ty,
            span,
        ))
        .with_id(id))
    }

    // -------------------------------------------------------------------------
    // C-FFI (`extern` Block & Single Extern Declaration)
    // -------------------------------------------------------------------------

    fn parse_extern_decl(&mut self, attributes: Vec<Attribute>, start: Span) -> ParseResult<Decl> {
        self.expect(TokenKind::Extern, "'extern'")?;
        let abi = self.parse_string_literal_node()?;

        if self.eat(TokenKind::OpenBrace).is_some() {
            let mut members = Vec::new();
            while !self.at(TokenKind::CloseBrace) && !self.is_eof() {
                if self.eat(TokenKind::Semicolon).is_some() {
                    continue;
                }
                let start_cursor = self.checkpoint().cursor;
                match self.parse_extern_member() {
                    Ok(member) => members.push(member),
                    Err(err) => {
                        self.report_error(err);
                        self.synchronize_from(start_cursor);
                    }
                }
            }
            self.expect(TokenKind::CloseBrace, "'}'")?;
            let span = self.span_from(start);
            let id = self.alloc_id();
            return Ok(
                Decl::extern_block(ExternBlockDecl::new(attributes, abi, members, span))
                    .with_id(id),
            );
        }

        self.expect(TokenKind::Function, "'function'")?;
        let name_tok = self.expect(TokenKind::Identifier, "extern function name")?;
        let name = Ident::new(self.slice(name_tok.span), name_tok.span);
        let type_parameters = if self.at(TokenKind::LessThan) {
            Some(self.parse_type_parameters()?)
        } else {
            None
        };
        let params = self.parse_extern_parameter_list()?;
        let return_type = if self.eat(TokenKind::Colon).is_some() {
            Some(self.parse_type()?)
        } else {
            None
        };
        let body = if self.at(TokenKind::OpenBrace) {
            Some(self.parse_block()?)
        } else {
            self.expect_eos_or_recover();
            None
        };

        let span = self.span_from(start);
        let id = self.alloc_id();
        Ok(Decl::single_extern(SingleExternDecl::new(
            attributes,
            abi,
            name,
            type_parameters,
            params,
            return_type,
            body,
            span,
        ))
        .with_id(id))
    }

    fn parse_extern_member(&mut self) -> ParseResult<ExternMember> {
        let start = self.current_span();
        let attributes = if self.at(TokenKind::HashBracket) {
            self.parse_attributes()?
        } else {
            Vec::new()
        };

        if self.eat(TokenKind::Function).is_some() {
            let name_tok = self.expect(TokenKind::Identifier, "extern function name")?;
            let name = Ident::new(self.slice(name_tok.span), name_tok.span);
            let type_parameters = if self.at(TokenKind::LessThan) {
                Some(self.parse_type_parameters()?)
            } else {
                None
            };
            let params = self.parse_extern_parameter_list()?;
            let return_type = if self.eat(TokenKind::Colon).is_some() {
                Some(self.parse_type()?)
            } else {
                None
            };
            self.expect_eos_or_recover();
            let span = self.span_from(start);
            return Ok(ExternMember::Function(ExternFunctionDecl {
                attributes,
                name,
                type_parameters,
                params,
                return_type,
                span,
            }));
        }

        if self.at(TokenKind::Let) || self.at(TokenKind::Const) {
            let is_const = self.bump().kind == TokenKind::Const;
            let name_tok = self.expect(TokenKind::Identifier, "extern variable name")?;
            let name = Ident::new(self.slice(name_tok.span), name_tok.span);
            self.expect(TokenKind::Colon, "':'")?;
            let ty = self.parse_type()?;
            self.expect_eos_or_recover();
            let span = self.span_from(start);
            return Ok(ExternMember::Variable(ExternVariableDecl {
                attributes,
                is_const,
                name,
                ty,
                span,
            }));
        }

        let tok = self.current();
        Err(ParseError::unexpected(
            "'function', 'let', or 'const' in extern block",
            tok.kind,
            tok.span,
        ))
    }

    fn parse_extern_parameter_list(&mut self) -> ParseResult<ExternParameterList> {
        let open = self.expect(TokenKind::OpenParen, "'('")?;
        let (params, is_variadic, rest) = self.with_grouping(|p| {
            let mut params = Vec::new();
            let mut is_variadic = false;
            let mut rest = None;

            while !p.at(TokenKind::CloseParen) && !p.is_eof() {
                if p.at(TokenKind::Ellipsis) {
                    if p.peek_kind(1) == TokenKind::Identifier {
                        rest = Some(p.parse_rest_parameter()?);
                    } else {
                        p.bump(); // consume '...'
                        is_variadic = true;
                    }
                    p.eat(TokenKind::Comma);
                    break;
                }

                params.push(p.parse_parameter()?);
                if p.eat(TokenKind::Comma).is_none() {
                    break;
                }
            }
            Ok::<_, ParseError>((params, is_variadic, rest))
        })?;

        self.expect(TokenKind::CloseParen, "')'")?;
        let span = self.span_from(open.span);
        Ok(ExternParameterList::new(params, is_variadic, rest, span))
    }

    // -------------------------------------------------------------------------
    // Parameters & Shared Helpers
    // -------------------------------------------------------------------------

    /// Parses a parenthesized parameter list: `(param1, param2, ...rest)`.
    pub fn parse_parameter_list(&mut self) -> ParseResult<ParameterList> {
        let open = self.expect(TokenKind::OpenParen, "'('")?;
        let params_res = self.with_grouping(|p| {
            let mut params = Vec::new();
            let mut rest = None;

            if !p.at(TokenKind::CloseParen) {
                loop {
                    if p.at(TokenKind::Ellipsis) {
                        rest = Some(p.parse_rest_parameter()?);
                        p.eat(TokenKind::Comma);
                        break;
                    }

                    params.push(p.parse_parameter()?);
                    if p.eat(TokenKind::Comma).is_none() || p.at(TokenKind::CloseParen) {
                        break;
                    }
                }
            }
            Ok::<_, ParseError>((params, rest))
        });

        let (params, rest) = params_res?;
        self.expect(TokenKind::CloseParen, "')'")?;
        let span = self.span_from(open.span);
        Ok(ParameterList::new(params, rest, span))
    }

    fn parse_parameter(&mut self) -> ParseResult<Parameter> {
        let start = self.current_span();
        let accessibility = self.parse_optional_accessibility();
        let is_readonly = self.eat(TokenKind::Readonly).is_some();
        let target = self.parse_binding_target()?;
        let is_optional = self.eat(TokenKind::Question).is_some();

        let ty = if self.eat(TokenKind::Colon).is_some() {
            Some(self.parse_type()?)
        } else {
            None
        };

        let default = if self.eat(TokenKind::Assign).is_some() {
            Some(self.parse_expr()?)
        } else {
            None
        };

        let span = self.span_from(start);
        Ok(Parameter::new(
            accessibility,
            is_readonly,
            target,
            is_optional,
            ty,
            default,
            span,
        ))
    }

    fn parse_rest_parameter(&mut self) -> ParseResult<RestParameter> {
        let ellipsis = self.expect(TokenKind::Ellipsis, "'...'")?;
        let name_tok = self.expect(TokenKind::Identifier, "rest parameter name")?;
        let name = Ident::new(self.slice(name_tok.span), name_tok.span);
        let ty = if self.eat(TokenKind::Colon).is_some() {
            Some(self.parse_type()?)
        } else {
            None
        };
        let span = self.span_from(ellipsis.span);
        Ok(RestParameter::new(name, ty, span))
    }

    /// Parses an optional `public` / `protected` / `private` accessibility modifier.
    pub(crate) fn parse_optional_accessibility(&mut self) -> Option<Accessibility> {
        let kind = match self.current_kind() {
            TokenKind::Public => Accessibility::Public,
            TokenKind::Protected => Accessibility::Protected,
            TokenKind::Private => Accessibility::Private,
            _ => return None,
        };
        self.bump();
        Some(kind)
    }

    fn parse_type_reference(&mut self) -> ParseResult<TypeReference> {
        let name = self.parse_type_name()?;
        let type_arguments = if self.at(TokenKind::LessThan) {
            Some(self.parse_type_arguments()?)
        } else {
            None
        };
        let span = self.span_from(name.span);
        Ok(TypeReference::new(name, type_arguments, span))
    }

    fn parse_type_reference_list(&mut self) -> ParseResult<Vec<TypeReference>> {
        let mut refs = vec![self.parse_type_reference()?];
        while self.eat(TokenKind::Comma).is_some() {
            refs.push(self.parse_type_reference()?);
        }
        Ok(refs)
    }

    fn parse_string_literal_node(&mut self) -> ParseResult<StringLiteral> {
        let tok = self.expect(TokenKind::StringLiteral, "string literal")?;
        let lit = self.parse_literal_token(tok)?;
        match lit.kind {
            LiteralKind::String(s) => Ok(s),
            _ => unreachable!("StringLiteral token always decodes to LiteralKind::String"),
        }
    }
}
