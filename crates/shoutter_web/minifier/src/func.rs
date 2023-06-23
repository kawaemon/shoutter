use swc_core::common::input::StringInput;
use swc_core::common::sync::Lrc;
use swc_core::common::{FileName, SourceMap};
use swc_core::ecma::ast::{
    ArrowExpr, BindingIdent, BlockStmtOrExpr, Decl, EsVersion, Expr, Function, Pat, Program,
    VarDecl, VarDeclKind, VarDeclarator,
};
use swc_core::ecma::codegen::text_writer::JsWriter;
use swc_core::ecma::codegen::Emitter;
use swc_core::ecma::parser::lexer::Lexer;
use swc_core::ecma::parser::Parser;
use swc_core::ecma::visit::{as_folder, FoldWith, VisitMut, VisitMutWith};

pub fn minify_function_decl(js: impl Into<String>) -> String {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("in.js".to_owned()), js.into());
    let mut parser = Parser::new_from(Lexer::new(
        Default::default(),
        EsVersion::latest(),
        StringInput::from(&*fm),
        None,
    ));
    let module = parser.parse_module().unwrap();
    let module = Program::Module(module)
        .fold_with(&mut as_folder(TransformVisitor))
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

fn map_function(f: Function) -> Option<ArrowExpr> {
    let Some(params) = f
        .params
        .into_iter()
        .map(|x| x.decorators.is_empty().then_some(x.pat))
        .collect::<Option<Vec<_>>>() else { return None };
    Some(ArrowExpr {
        span: f.span,
        params,
        body: Box::new(BlockStmtOrExpr::BlockStmt(f.body.unwrap())),
        is_async: f.is_async,
        is_generator: f.is_generator,
        type_params: f.type_params,
        return_type: f.return_type,
    })
}

pub struct TransformVisitor;

impl VisitMut for TransformVisitor {
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
                type_ann: None, // what is this
            }),
            init: Some(Box::new(Expr::Arrow(arrow_fn))),
            definite: false, // what is this
        };

        *n = Decl::Var(Box::new(VarDecl {
            span,
            kind: VarDeclKind::Const,
            declare: f.declare,
            decls: vec![d],
        }));
    }
}
