use std::{
    collections::{BTreeMap, BTreeSet},
    mem::{replace, take},
};


use swc_atoms::{Atom, Wtf8Atom};
use swc_common::{Span, Spanned};
use swc_ecma_ast::{
    ArrayLit, ArrowExpr, AssignExpr, AssignOp, AssignTarget, BinExpr, BinaryOp, BlockStmt,
    BlockStmtOrExpr, CallExpr, Callee, ComputedPropName, Decl, Expr, ExprOrSpread, ExprStmt,
    Function, Id, Ident, IdentName, IfStmt, Lit, MemberExpr, MemberProp, ModuleDecl, ModuleItem,
    Param, SimpleAssignTarget, Stmt, Str, YieldExpr,
};
use swc_ecma_visit::{Visit, VisitMut, VisitMutWith, VisitWith};
pub mod amd;