use std::collections::{HashMap, HashSet};

use sha2::{Digest, Sha256};
use swc_core::common::input::StringInput;
use swc_core::common::sync::Lrc;
use swc_core::common::{FileName, SourceMap, DUMMY_SP};
use swc_core::ecma::ast::{
    ArrowExpr, BindingIdent, BlockStmt, BlockStmtOrExpr, CallExpr, Decl, EsVersion, Expr,
    ExprOrSpread, FnDecl, FnExpr, Function, Ident, Lit, Module, ModuleItem, Param, Pat, Program,
    RestPat, ReturnStmt, Stmt, Str, VarDecl, VarDeclKind, VarDeclarator,
};
use swc_core::ecma::atoms::JsWord;
use swc_core::ecma::codegen::text_writer::JsWriter;
use swc_core::ecma::codegen::Emitter;
use swc_core::ecma::parser::lexer::Lexer;
use swc_core::ecma::parser::Parser;
use swc_core::ecma::visit::{as_folder, FoldWith, Visit, VisitMut, VisitMutWith, VisitWith};

pub fn optimize_js(js: impl Into<String>) -> String {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("in.js".to_owned()), js.into());
    let module = Parser::new_from(Lexer::new(
        Default::default(),
        EsVersion::latest(),
        StringInput::from(&*fm),
        None,
    ))
    .parse_module()
    .unwrap();
    let module = Program::Module(module)
        .fold_with(&mut as_folder(FunctionToArrowFn))
        // worse
        // .fold_with(&mut as_folder(InternString))
        .expect_module();
    let mut buf = vec![];
    Emitter {
        cfg: Default::default(),
        cm: cm.clone(),
        comments: Default::default(),
        wr: Box::new(JsWriter::new(cm, "\n", &mut buf, None)),
    }
    .emit_module(&module)
    .unwrap();
    String::from_utf8(buf).unwrap()
}

fn map_function(mut f: Function) -> Option<ArrowExpr> {
    // replace `arguments` special identifier to rest parameter
    // from: function() { d(arguments); }
    // to  : function(...a) { d(a); }
    let arg_replacement = JsWord::from("__minifier_arguments");
    let mut arg_replacer = RenameArguments::new(arg_replacement.clone());
    f.body.visit_mut_children_with(&mut arg_replacer);
    if arg_replacer.have_arguments {
        if !f.params.is_empty() {
            panic!("have_arguments && !params.is_empty");
        }
        f.params.push(Param {
            pat: Pat::Rest(RestPat {
                arg: Box::new(Pat::Ident(BindingIdent {
                    id: Ident::new(arg_replacement, DUMMY_SP),
                    type_ann: None,
                })),
                span: DUMMY_SP,
                dot3_token: DUMMY_SP,
                type_ann: None,
            }),
            span: DUMMY_SP,
            decorators: vec![],
        })
    }

    // decorator is not allowed on arrow function.
    // e.g. rejects: function(@a hoge) {}
    let params = f
        .params
        .into_iter()
        .map(|x| x.decorators.is_empty().then_some(x.pat))
        .collect::<Option<Vec<_>>>()?;
    let mut arrow = ArrowExpr {
        span: f.span,
        params,
        body: Box::new(BlockStmtOrExpr::BlockStmt(f.body.unwrap())),
        is_async: f.is_async,
        is_generator: f.is_generator,
        type_params: f.type_params,
        return_type: f.return_type,
    };

    // from: () => { const arg_ident = init; return arg_ident; }
    // to  : () => init;
    // if let Some(BlockStmt { stmts: body, .. }) = &mut f.body
    if let BlockStmtOrExpr::BlockStmt(BlockStmt { stmts: body, .. }) = &mut *arrow.body
        && let [may_decl, may_ret] = &mut body[..]
        && let Stmt::Decl(Decl::Var(box VarDecl { kind: VarDeclKind::Const, declare: false, decls, span: _  })) = may_decl
        && let [VarDeclarator { name: Pat::Ident(BindingIdent { id: ref decl_name, type_ann: None }), init: Some(ref init), definite: false, .. }] = decls[..]
        && let Stmt::Return(ReturnStmt { arg: Some(box Expr::Call(CallExpr { args, type_args: None, .. })), .. }) = may_ret
        && let [ExprOrSpread { expr: box ref mut arg, .. }] = args[..]
        && let Expr::Ident(arg_ident) = arg
        && arg_ident.sym == decl_name.sym
    {
        arrow.body = Box::new(BlockStmtOrExpr::Expr(init.clone()));
    }

    Some(arrow)
}

pub struct FunctionToArrowFn;

impl VisitMut for FunctionToArrowFn {
    fn visit_mut_expr(&mut self, n: &mut Expr) {
        n.visit_mut_children_with(self);
        let Expr::Fn(f) = n else { return };

        if f.ident.is_some() {
            return;
        }

        let Some(arrow_fn) = map_function(*f.function.clone()) else { return };
        *n = Expr::Arrow(arrow_fn);
    }

    fn visit_mut_decl(&mut self, n: &mut Decl) {
        n.visit_mut_children_with(self);

        let Decl::Fn(f) = n else { return };

        let Some(arrow_fn) = map_function(*f.function.clone()) else { return };

        let span = f.function.span;
        let d = VarDeclarator {
            span,
            name: Pat::Ident(BindingIdent {
                id: f.ident.clone(),
                type_ann: None,
            }),
            init: Some(Box::new(Expr::Arrow(arrow_fn))),
            definite: false,
        };

        *n = Decl::Var(Box::new(VarDecl {
            span,
            kind: VarDeclKind::Const,
            declare: f.declare,
            decls: vec![d],
        }));
    }
}

/// find `arguments` identifier
pub struct RenameArguments {
    replacement: JsWord,
    have_arguments: bool,
}
impl RenameArguments {
    fn new(replacement: JsWord) -> Self {
        Self {
            replacement,
            have_arguments: false,
        }
    }
}
impl VisitMut for RenameArguments {
    fn visit_mut_fn_decl(&mut self, _n: &mut FnDecl) {
        // stop propergation
    }
    fn visit_mut_fn_expr(&mut self, _n: &mut FnExpr) {
        // stop propergation
    }
    fn visit_mut_ident(&mut self, n: &mut Ident) {
        if &*n.sym == "arguments" {
            n.sym = self.replacement.clone();
            self.have_arguments = true;
        }
    }
}

pub struct InternString;

impl InternString {
    fn stored_str_referrer(&self, s: &JsWord) -> JsWord {
        let mut hasher = Sha256::new();
        hasher.update(s.as_bytes());
        let hash = hex::encode(hasher.finalize());
        JsWord::from(format!("__minifier_interned_str_{}", &hash[0..8]))
    }
}

impl VisitMut for InternString {
    fn visit_mut_module(&mut self, m: &mut Module) {
        let mut counter = CountStringLiteral::default();
        m.visit_children_with(&mut counter);

        let mut must_define = HashSet::new();

        const INTERN_THRESHOLD: usize = 3;
        let mut replacer = ReplaceStringLiteral::new(|lit| {
            if counter.count.get(lit).copied().unwrap_or(0) < INTERN_THRESHOLD {
                return None;
            }
            must_define.insert(lit.clone());
            Some(self.stored_str_referrer(lit))
        });
        m.visit_mut_children_with(&mut replacer);

        m.body.insert(
            0,
            ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
                span: DUMMY_SP,
                kind: VarDeclKind::Const,
                declare: false,
                decls: must_define
                    .into_iter()
                    .map(|lit| VarDeclarator {
                        span: DUMMY_SP,
                        name: Pat::Ident(BindingIdent {
                            id: Ident {
                                span: DUMMY_SP,
                                sym: self.stored_str_referrer(&lit),
                                optional: false,
                            },
                            type_ann: None,
                        }),
                        init: Some(Box::new(Expr::Lit(Lit::Str(Str {
                            span: DUMMY_SP,
                            value: lit,
                            raw: None,
                        })))),
                        definite: false,
                    })
                    .collect(),
            })))),
        );
    }
}

#[derive(Default)]
pub struct CountStringLiteral {
    count: HashMap<JsWord, usize>,
}

impl Visit for CountStringLiteral {
    fn visit_str(&mut self, s: &Str) {
        *self.count.entry(s.value.clone()).or_insert(0) += 1;
    }
}

pub struct ReplaceStringLiteral<F> {
    should_replace: F,
}
impl<F> ReplaceStringLiteral<F>
where
    F: FnMut(&JsWord) -> Option<JsWord>,
{
    fn new(should_replace: F) -> Self {
        Self { should_replace }
    }
}

impl<F> VisitMut for ReplaceStringLiteral<F>
where
    F: FnMut(&JsWord) -> Option<JsWord>,
{
    fn visit_mut_expr(&mut self, expr: &mut Expr) {
        let Expr::Lit(Lit::Str(lit)) = expr else {
            expr.visit_mut_children_with(self);
            return;
        };

        let Some(rep_ident) = (self.should_replace)(&lit.value) else { return };
        *expr = Expr::Ident(Ident {
            span: DUMMY_SP,
            sym: rep_ident,
            optional: false,
        });
    }
}
