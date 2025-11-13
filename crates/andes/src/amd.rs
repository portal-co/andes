use super::*;

// use swc_ecma_parser::token::IdentKind;
pub struct AMD {
    modules: ArrayLit,
    body: BlockStmt,
    params: Vec<Ident>,
    imports: BTreeMap<Atom, Ident>,
    exports: BTreeSet<Wtf8Atom>,
    awaiter_flag: bool,
}
impl AMD {
    fn get_import(&mut self, a: Atom, span: Span) -> Ident {
        return self
            .imports
            .entry(a)
            .or_insert_with_key(|a| {
                self.modules.elems.push(Some(ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Lit(Lit::Str(Str {
                        span,
                        value: a.clone().into(),
                        raw: None,
                    }))),
                }));
                let id = Ident::new_private(a.clone(), span);
                self.params.push(id.clone().into());
                return id;
            })
            .clone()
            .with_pos(span.lo, span.hi);
    }
    fn collect_imports(a: &mut Vec<ModuleItem>) -> BTreeMap<Id, (Wtf8Atom, Option<Wtf8Atom>)> {
        let mut m: BTreeMap<Id, (Wtf8Atom, Option<Wtf8Atom>)> = BTreeMap::new();
        for a2 in take(a) {
            let a2 = match a2 {
                ModuleItem::ModuleDecl(d) => match d {
                    ModuleDecl::Import(i) => {
                        let src = i.src.value.clone();
                        for s in i.specifiers.iter() {
                            let (id, dest) = match s {
                                swc_ecma_ast::ImportSpecifier::Named(import_named_specifier) => (
                                    import_named_specifier.local.clone(),
                                    Some(match &import_named_specifier.imported {
                                        None => import_named_specifier.local.sym.clone().into(),
                                        Some(i) => match i {
                                            swc_ecma_ast::ModuleExportName::Ident(ident) => {
                                                ident.sym.clone().into()
                                            }
                                            swc_ecma_ast::ModuleExportName::Str(s) => {
                                                s.value.clone()
                                            }
                                        },
                                    }),
                                ),
                                swc_ecma_ast::ImportSpecifier::Default(
                                    import_default_specifier,
                                ) => (
                                    import_default_specifier.local.clone(),
                                    Some(Wtf8Atom::new("default")),
                                ),
                                swc_ecma_ast::ImportSpecifier::Namespace(
                                    import_star_as_specifier,
                                ) => (import_star_as_specifier.local.clone(), None),
                            };
                            m.insert(id.to_id(), (src.clone(), dest));
                        }
                        continue;
                    }
                    d => ModuleItem::ModuleDecl(d),
                },
                a2 => a2,
            };
            a.push(a2);
        }
        return m;
    }
}
struct AMDPass<'a> {
    amd: &'a mut AMD,
    idents: BTreeMap<Id, (Atom, Option<Atom>)>,
    in_func: bool,
}
impl<'a> VisitMut for AMDPass<'a> {
    fn visit_mut_expr(&mut self, node: &mut Expr) {
        node.visit_mut_children_with(self);
        if let Expr::Ident(i) = node
            && let Some((a, b)) = self.idents.get(&i.to_id())
        {
            let id = self.amd.get_import(a.clone(), node.span());
            *node = match b {
                None => id.into(),
                Some(b) => Expr::Member(MemberExpr {
                    span: node.span(),
                    obj: id.into(),
                    prop: MemberProp::Computed(ComputedPropName {
                        span: node.span(),
                        expr: Box::new(Expr::Lit(Lit::Str(Str {
                            span: node.span(),
                            value: b.clone().into(),
                            raw: None,
                        }))),
                    }),
                }),
            }
        }
        if let Expr::Await(a) = node
            && !self.in_func
        {
            self.amd.awaiter_flag = true;
            *node = Expr::Yield(YieldExpr {
                span: a.span,
                arg: Some(take(&mut a.arg)),
                delegate: false,
            })
        }
    }
    fn visit_mut_function(&mut self, node: &mut Function) {
        let old = replace(&mut self.in_func, true);
        node.visit_mut_children_with(self);
        self.in_func = old;
    }
    fn visit_mut_arrow_expr(&mut self, node: &mut ArrowExpr) {
        let old = replace(&mut self.in_func, true);
        node.visit_mut_children_with(self);
        self.in_func = old;
    }
    fn visit_mut_module_items(&mut self, node: &mut Vec<ModuleItem>) {
        for item in take(node) {
            match item {
                ModuleItem::ModuleDecl(module_decl) => match module_decl {
                    ModuleDecl::Import(import_decl) => todo!(),
                    ModuleDecl::ExportDecl(export_decl) => {
                        let name = match &export_decl.decl {
                            Decl::Class(class_decl) => vec![class_decl.ident.clone()],
                            Decl::Fn(fn_decl) => vec![fn_decl.ident.clone()],
                            Decl::Var(var_decl) => {
                                struct IdentLister {
                                    x: Vec<Ident>,
                                }
                                impl Visit for IdentLister {
                                    fn visit_ident(&mut self, node: &Ident) {
                                        self.x.push(node.clone());
                                    }
                                }
                                let mut l = IdentLister {
                                    x: Default::default(),
                                };
                                var_decl.visit_with(&mut l);
                                l.x
                            }
                            Decl::Using(using_decl) => todo!(),
                            Decl::TsInterface(ts_interface_decl) => todo!(),
                            Decl::TsTypeAlias(ts_type_alias_decl) => todo!(),
                            Decl::TsEnum(ts_enum_decl) => todo!(),
                            Decl::TsModule(ts_module_decl) => todo!(),
                        };
                        node.push(ModuleItem::Stmt(Stmt::Decl(export_decl.decl)));
                        let exports = self.amd.get_import(Atom::new("exports"), export_decl.span);
                        for n in name {
                            self.amd.exports.insert(n.sym.clone().into());
                            node.push(ModuleItem::Stmt(Stmt::Expr(ExprStmt {
                                span: export_decl.span,
                                expr: Box::new(Expr::Assign(AssignExpr {
                                    span: export_decl.span,
                                    op: AssignOp::Assign,
                                    left: AssignTarget::Simple(SimpleAssignTarget::Member(
                                        MemberExpr {
                                            span: export_decl.span,
                                            obj: exports.clone().into(),
                                            prop: MemberProp::Ident(IdentName {
                                                span: n.span,
                                                sym: n.sym.clone(),
                                            }),
                                        },
                                    )),
                                    right: n.clone().into(),
                                })),
                            })))
                        }
                    }
                    ModuleDecl::ExportNamed(named_export) => {
                        let exports = self.amd.get_import(Atom::new("exports"), named_export.span);
                        for s in named_export.specifiers {
                            let (id, x) = match s {
                                swc_ecma_ast::ExportSpecifier::Namespace(
                                    export_namespace_specifier,
                                ) => todo!(),
                                swc_ecma_ast::ExportSpecifier::Default(
                                    export_default_specifier,
                                ) => (
                                    export_default_specifier.exported.clone(),
                                    Wtf8Atom::new("default"),
                                ),
                                swc_ecma_ast::ExportSpecifier::Named(export_named_specifier) => {
                                    let id = match export_named_specifier.orig {
                                        swc_ecma_ast::ModuleExportName::Ident(ident) => ident,
                                        swc_ecma_ast::ModuleExportName::Str(_) => todo!(),
                                    };
                                    let x = export_named_specifier
                                        .exported
                                        .as_ref()
                                        .map(|a| match a {
                                            swc_ecma_ast::ModuleExportName::Ident(ident) => {
                                                ident.sym.clone().into()
                                            }
                                            swc_ecma_ast::ModuleExportName::Str(s) => {
                                                s.value.clone()
                                            }
                                        })
                                        .unwrap_or_else(|| id.sym.clone().into());
                                    (id, x)
                                }
                            };
                            self.amd.exports.insert(x.clone());
                            node.push(ModuleItem::Stmt(Stmt::Expr(ExprStmt {
                                span: named_export.span,
                                expr: Box::new(Expr::Assign(AssignExpr {
                                    span: named_export.span,
                                    op: AssignOp::Assign,
                                    left: AssignTarget::Simple(SimpleAssignTarget::Member(
                                        MemberExpr {
                                            span: named_export.span,
                                            obj: exports.clone().into(),
                                            prop: MemberProp::Ident(IdentName {
                                                span: id.span,
                                                sym: (&*x.to_atom_lossy()).clone(),
                                            }),
                                        },
                                    )),
                                    right: id.into(),
                                })),
                            })))
                        }
                    }
                    ModuleDecl::ExportDefaultDecl(mut export_default_decl) => {
                        let name = match &mut export_default_decl.decl {
                            swc_ecma_ast::DefaultDecl::Class(class_expr) => class_expr
                                .ident
                                .get_or_insert_with(|| {
                                    Ident::new_private(Atom::new("Class"), export_default_decl.span)
                                })
                                .clone(),
                            swc_ecma_ast::DefaultDecl::Fn(fn_expr) => fn_expr
                                .ident
                                .get_or_insert_with(|| {
                                    Ident::new_private(Atom::new("fn"), export_default_decl.span)
                                })
                                .clone(),
                            swc_ecma_ast::DefaultDecl::TsInterfaceDecl(ts_interface_decl) => {
                                todo!()
                            }
                        };
                        node.push(ModuleItem::Stmt(Stmt::Expr(ExprStmt {
                            span: export_default_decl.span,
                            expr: Box::new(match export_default_decl.decl {
                                swc_ecma_ast::DefaultDecl::Class(class_expr) => {
                                    Expr::Class(class_expr)
                                }
                                swc_ecma_ast::DefaultDecl::Fn(fn_expr) => Expr::Fn(fn_expr),
                                swc_ecma_ast::DefaultDecl::TsInterfaceDecl(ts_interface_decl) => {
                                    todo!()
                                }
                            }),
                        })));
                        let exports = self
                            .amd
                            .get_import(Atom::new("exports"), export_default_decl.span);
                        self.amd.exports.insert(Wtf8Atom::new("default"));
                        node.push(ModuleItem::Stmt(Stmt::Expr(ExprStmt {
                            span: export_default_decl.span,
                            expr: Box::new(Expr::Assign(AssignExpr {
                                span: export_default_decl.span,
                                op: AssignOp::Assign,
                                left: AssignTarget::Simple(SimpleAssignTarget::Member(
                                    MemberExpr {
                                        span: export_default_decl.span,
                                        obj: exports.clone().into(),
                                        prop: MemberProp::Ident(IdentName {
                                            span: export_default_decl.span,
                                            sym: Atom::new("default"),
                                        }),
                                    },
                                )),
                                right: name.into(),
                            })),
                        })))
                    }
                    ModuleDecl::ExportDefaultExpr(export_default_expr) => {
                        let exports = self
                            .amd
                            .get_import(Atom::new("exports"), export_default_expr.span);
                        self.amd.exports.insert(Wtf8Atom::new("default"));
                        node.push(ModuleItem::Stmt(Stmt::Expr(ExprStmt {
                            span: export_default_expr.span,
                            expr: Box::new(Expr::Assign(AssignExpr {
                                span: export_default_expr.span,
                                op: AssignOp::Assign,
                                left: AssignTarget::Simple(SimpleAssignTarget::Member(
                                    MemberExpr {
                                        span: export_default_expr.span,
                                        obj: exports.clone().into(),
                                        prop: MemberProp::Ident(IdentName {
                                            span: export_default_expr.span,
                                            sym: Atom::new("default"),
                                        }),
                                    },
                                )),
                                right: export_default_expr.expr,
                            })),
                        })))
                    }
                    ModuleDecl::ExportAll(export_all) => todo!(),
                    ModuleDecl::TsImportEquals(ts_import_equals_decl) => todo!(),
                    ModuleDecl::TsExportAssignment(ts_export_assignment) => todo!(),
                    ModuleDecl::TsNamespaceExport(ts_namespace_export_decl) => todo!(),
                },
                ModuleItem::Stmt(stmt) => node.push(ModuleItem::Stmt(stmt)),
            };
        }
        node.visit_mut_children_with(self);
    }
}
