#![allow(dead_code)]

use std::collections::HashMap;
use std::path::Path;

use minifier_rs_macro::struct_map;
use wasm_encoder::{ConstExpr, ElementSegment};

macro_rules! enum_map {
    () => {};
    (fn $fn_name:ident($ty:ident) { $($tt:tt)* } $($tail:tt)*) => {
        fn $fn_name(a: wasmparser::$ty) -> wasm_encoder::$ty {
            enum_map!(@arm [a $ty $ty] $($tt)*);
            unreachable!();
        }
        enum_map!($($tail)*);
    };
    (fn $fn_name:ident($from_ty:ident) -> $to_ty:ident { $($tt:tt)* } $($tail:tt)*) => {
        fn $fn_name(a: wasmparser::$from_ty) -> wasm_encoder::$to_ty {
            enum_map!(@arm [a $from_ty $to_ty] $($tt)*);
            unreachable!();
        }
        enum_map!($($tail)*);
    };

    (@arm [$var:ident $from_ty:ident $to_ty:ident]) => {};
    (@arm [$var:ident $from_ty:ident $to_ty:ident] $name:ident$(($($params:ident),*$(,)?))? => $block:block, $($tail:tt)*) => {
        if let wasmparser::$from_ty::$name$(($($params)*))? = $var {
            return $block;
        }
        enum_map!(@arm [$var $from_ty $to_ty] $($tail)*);
    };
    (@arm [$var:ident $from_ty:ident $to_ty:ident] $name:ident$(($($params:ident),*$(,)?))?, $($tail:tt)*) => {
        #[allow(irrefutable_let_patterns)]
        if let wasmparser::$from_ty::$name$(($($params)*))? = $var {
            return wasm_encoder::$to_ty::$name$(($($params)*))?;
        }
        enum_map!(@arm [$var $from_ty $to_ty] $($tail)*);
    };
}

enum_map! {
    fn map_heap_type(HeapType) {
        Func, Extern, Any, None, NoExtern, NoFunc, Eq, Struct, Array, I31, Indexed(i),
    }

    fn map_val_type(ValType) {
        I32, I64, F32, F64, V128, Ref(ref_) => {
            wasm_encoder::ValType::Ref(map_ref_type(ref_))
        },
    }
    fn map_storage_type(StorageType) {
        I8, I16, Val(val) => {
            wasm_encoder::StorageType::Val(map_val_type(val))
        },
    }
    fn map_tag_kind(TagKind) {
        Exception,
    }
    fn map_external_kind(ExternalKind) -> ExportKind {
        Func, Table, Memory, Global, Tag,
    }
}
struct_map! {
    fn map_table_type(t: TableType) {
        element_type: { map_ref_type(t.element_type) },
        minimum: initial,
        maximum,
    }
    fn map_memory_type(m: MemoryType) {
        minimum: initial,
        maximum, memory64, shared,
    }
    fn map_global_type(g: GlobalType) {
        val_type: { map_val_type(g.content_type) },
        mutable,
    }
    fn map_tag_type(t: TagType) {
        kind: { map_tag_kind(t.kind) },
        func_type_idx,
    }
}

fn map_const_expr(c: wasmparser::ConstExpr) -> wasm_encoder::ConstExpr {
    let mut reader = c.get_binary_reader();
    // wasm_encoder::ConstExpr appends Instruction::End at last
    let bytes = reader.read_bytes(reader.bytes_remaining() - 1).unwrap();
    wasm_encoder::ConstExpr::raw(bytes.iter().copied())
}
fn map_element_items<'a>(
    items: wasmparser::ElementItems,
    functions: &'a mut Vec<u32>,
    const_exprs: &'a mut Vec<wasm_encoder::ConstExpr>,
) -> wasm_encoder::Elements<'a> {
    match items {
        wasmparser::ElementItems::Functions(f) => {
            functions.extend(f.into_iter().map(|x| x.unwrap()));
            wasm_encoder::Elements::Functions(functions)
        }
        wasmparser::ElementItems::Expressions(e) => {
            const_exprs.extend(e.into_iter().map(|x| map_const_expr(x.unwrap())));
            wasm_encoder::Elements::Expressions(const_exprs)
        }
    }
}
fn map_element_kind<'a>(
    e: wasmparser::ElementKind,
    offset: &'a mut Option<ConstExpr>, // just for storage. should be None
) -> wasm_encoder::ElementMode<'a> {
    match e {
        wasmparser::ElementKind::Passive => wasm_encoder::ElementMode::Passive,
        wasmparser::ElementKind::Active {
            table_index,
            offset_expr,
        } => wasm_encoder::ElementMode::Active {
            table: table_index,
            offset: {
                offset.replace(map_const_expr(offset_expr));
                offset.as_ref().unwrap()
            },
        },
        wasmparser::ElementKind::Declared => wasm_encoder::ElementMode::Declared,
    }
}
fn map_ref_type(ref_: wasmparser::RefType) -> wasm_encoder::RefType {
    wasm_encoder::RefType {
        nullable: ref_.is_nullable(),
        heap_type: map_heap_type(ref_.heap_type()),
    }
}

pub async fn minify_symbol(wasm_path: &Path, js_path: &Path) {
    let wasm = crate::fs::read_file(wasm_path).await.unwrap();
    let parser = wasmparser::Parser::new(0);

    let mut module = wasm_encoder::Module::new();
    let mut imports_ident_map = HashMap::new();
    let mut exports_ident_map = HashMap::new();

    let mut module_ident = MinifiedIdent::new();
    let mut name_ident = MinifiedIdent::new();
    let mut export_ident = MinifiedIdent::new();

    let mut code_section_remaining = 0;
    let mut code_section_encoder = None;

    for payload in parser.parse_all(&wasm) {
        let payload = payload.unwrap();
        match payload {
            wasmparser::Payload::TypeSection(section) => {
                let mut encoder = wasm_encoder::TypeSection::new();
                for ty in section {
                    match ty.unwrap() {
                        wasmparser::Type::Func(f) => encoder.function(
                            f.params().iter().copied().map(map_val_type),
                            f.results().iter().copied().map(map_val_type),
                        ),
                        wasmparser::Type::Array(a) => {
                            encoder.array(map_storage_type(a.element_type), a.mutable)
                        }
                    };
                }
                module.section(&encoder);
            }
            wasmparser::Payload::ImportSection(section) => {
                let mut encoder = wasm_encoder::ImportSection::new();
                for import in section {
                    let import = import.unwrap();
                    let (module_name, name_map) = imports_ident_map
                        .entry(import.module)
                        .or_insert_with(|| (module_ident.next().unwrap(), HashMap::new()));
                    let name = name_map
                        .entry(import.name)
                        .or_insert_with(|| name_ident.next().unwrap());
                    encoder.import(
                        module_name,
                        name,
                        match import.ty {
                            wasmparser::TypeRef::Func(f) => wasm_encoder::EntityType::Function(f),
                            wasmparser::TypeRef::Table(t) => {
                                wasm_encoder::EntityType::Table(map_table_type(t))
                            }
                            wasmparser::TypeRef::Memory(m) => {
                                wasm_encoder::EntityType::Memory(map_memory_type(m))
                            }
                            wasmparser::TypeRef::Global(g) => {
                                wasm_encoder::EntityType::Global(map_global_type(g))
                            }
                            wasmparser::TypeRef::Tag(t) => {
                                wasm_encoder::EntityType::Tag(map_tag_type(t))
                            }
                        },
                    );
                }
                module.section(&encoder);
            }
            wasmparser::Payload::FunctionSection(section) => {
                let mut encoder = wasm_encoder::FunctionSection::new();
                for function in section {
                    encoder.function(function.unwrap());
                }
                module.section(&encoder);
            }
            wasmparser::Payload::TableSection(section) => {
                let mut encoder = wasm_encoder::TableSection::new();
                for table in section {
                    encoder.table(map_table_type(table.unwrap().ty));
                }
                module.section(&encoder);
            }
            wasmparser::Payload::MemorySection(section) => {
                let mut encoder = wasm_encoder::MemorySection::new();
                for memory in section {
                    encoder.memory(map_memory_type(memory.unwrap()));
                }
                module.section(&encoder);
            }
            wasmparser::Payload::TagSection(section) => {
                let mut encoder = wasm_encoder::TagSection::new();
                for tag in section {
                    encoder.tag(map_tag_type(tag.unwrap()));
                }
                module.section(&encoder);
            }
            wasmparser::Payload::GlobalSection(section) => {
                let mut encoder = wasm_encoder::GlobalSection::new();
                for global in section {
                    let global = global.unwrap();
                    encoder.global(
                        map_global_type(global.ty),
                        &map_const_expr(global.init_expr),
                    );
                }
                module.section(&encoder);
            }
            wasmparser::Payload::ExportSection(section) => {
                let mut encoder = wasm_encoder::ExportSection::new();
                for export in section {
                    let export = export.unwrap();
                    let export_name = exports_ident_map
                        .entry(export.name)
                        .or_insert_with(|| export_ident.next().unwrap());
                    encoder.export(export_name, map_external_kind(export.kind), export.index);
                }
                module.section(&encoder);
            }

            wasmparser::Payload::ElementSection(section) => {
                let mut encoder = wasm_encoder::ElementSection::new();
                for element in section {
                    let element = element.unwrap();
                    let (mut offset, mut functions, mut const_exprs) = (None, vec![], vec![]);
                    let segment = ElementSegment {
                        mode: map_element_kind(element.kind, &mut offset),
                        element_type: map_ref_type(element.ty),
                        elements: map_element_items(
                            element.items,
                            &mut functions,
                            &mut const_exprs,
                        ),
                    };
                    encoder.segment(segment);
                }
                module.section(&encoder);
            }

            wasmparser::Payload::DataSection(section) => {
                let mut encoder = wasm_encoder::DataSection::new();
                for data in section {
                    let data = data.unwrap();
                    match data.kind {
                        wasmparser::DataKind::Passive => {
                            encoder.passive(data.data.iter().copied());
                        }
                        wasmparser::DataKind::Active {
                            memory_index,
                            offset_expr,
                        } => {
                            encoder.active(
                                memory_index,
                                &map_const_expr(offset_expr),
                                data.data.iter().copied(),
                            );
                        }
                    }
                }
                module.section(&encoder);
            }

            wasmparser::Payload::CustomSection(section) => {
                module.section(&wasm_encoder::CustomSection {
                    name: section.name().into(),
                    data: section.data().into(),
                });
            }

            wasmparser::Payload::CodeSectionStart { count, .. } => {
                assert_eq!(code_section_remaining, 0);
                code_section_remaining = count;
                code_section_encoder = Some(wasm_encoder::CodeSection::new());
            }

            wasmparser::Payload::CodeSectionEntry(f) => {
                let mut reader = f.get_binary_reader();
                let bytes = reader.read_bytes(reader.bytes_remaining()).unwrap();

                let mut function = wasm_encoder::Function::new([]);

                pub struct Function {
                    bytes: Vec<u8>,
                }
                unsafe {
                    (*(&function as *const _ as *const Function as *mut Function))
                        .bytes
                        .clear();
                }
                assert_eq!(function.byte_len(), 0);

                function.raw(bytes.iter().copied());

                let encoder = code_section_encoder.as_mut().unwrap();
                encoder.function(&function);

                code_section_remaining -= 1;
                if code_section_remaining == 0 {
                    module.section(encoder);
                    code_section_encoder = None;
                }
            }

            wasmparser::Payload::Version { .. } | wasmparser::Payload::End(_) => {}

            e @ (wasmparser::Payload::StartSection { .. }
            | wasmparser::Payload::InstanceSection(_)
            | wasmparser::Payload::CoreTypeSection(_)
            | wasmparser::Payload::UnknownSection { .. }
            | wasmparser::Payload::DataCountSection { .. }
            | wasmparser::Payload::ModuleSection { .. }
            | wasmparser::Payload::ComponentSection { .. }
            | wasmparser::Payload::ComponentInstanceSection(_)
            | wasmparser::Payload::ComponentAliasSection(_)
            | wasmparser::Payload::ComponentTypeSection(_)
            | wasmparser::Payload::ComponentCanonicalSection(_)
            | wasmparser::Payload::ComponentStartSection { .. }
            | wasmparser::Payload::ComponentImportSection(_)
            | wasmparser::Payload::ComponentExportSection(_)) => todo!("{e:#?}"),
        }
    }

    assert!(code_section_encoder.is_none());

    let new_wasm = module.finish();

    crate::fs::write_file(wasm_path, &new_wasm).await.unwrap();

    let js = crate::fs::read_file(js_path).await.unwrap();
    let mut js = String::from_utf8(js).unwrap();

    // drawback: modifing javascript AST is better
    for (mod_before, (mod_after, fn_idents)) in imports_ident_map {
        js = js.replace(
            &format!("imports.{mod_before} = {{}};"),
            &format!("imports.{mod_after} = {{}};"),
        );

        for (fn_before, fn_after) in fn_idents {
            js = js.replace(
                &format!("imports.{mod_before}.{fn_before}"),
                &format!("imports.{mod_after}.{fn_after}"),
            );
        }
    }
    for (export_before, export_after) in exports_ident_map {
        js = js.replace(
            &format!("wasm.{export_before}"),
            &format!("wasm.{export_after}"),
        );
    }

    crate::fs::write_file(js_path, js.as_bytes()).await.unwrap();
}

struct MinifiedIdent {
    n: usize,
}
impl MinifiedIdent {
    fn new() -> Self {
        MinifiedIdent { n: 0 }
    }
}
impl Iterator for MinifiedIdent {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let mut ret = String::new();
        let chars = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let mut n = self.n;
        loop {
            ret.push(chars[n % chars.len()] as char);
            n /= chars.len();
            if n == 0 {
                break;
            }
        }
        self.n += 1;
        Some(ret)
    }
}
#[test]
fn minified_ident() {
    assert_eq!(
        MinifiedIdent::new().take(5).collect::<Vec<_>>().join(""),
        "abcde"
    );
    assert_eq!(
        MinifiedIdent::new()
            .step_by(10)
            .take(10)
            .collect::<Vec<_>>()
            .join(" "),
        "a k u E O Y ib sb Cb Mb"
    );
}
