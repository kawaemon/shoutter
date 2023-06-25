#![feature(stmt_expr_attributes)]
#![feature(let_chains)]
#![feature(box_patterns)]

mod opt_js;
mod symbol;
mod sys;

use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use anyhow::Result;
use once_cell::sync::Lazy;
use tracing::{Metadata, Subscriber};
use tracing_subscriber::fmt::format::{FmtSpan, Pretty};
use tracing_subscriber::fmt::time::UtcTime;
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use web_sys::console;

use crate::sys::{brotli, fs};

#[wasm_bindgen(start)]
async fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    start().await.unwrap();
}

static ORIGINAL_DIR: Lazy<&Path> = Lazy::new(|| Path::new("../../dist"));
static MINIFIED_DIR: Lazy<&Path> = Lazy::new(|| Path::new("../../dist-minified"));

struct ProcessStats {
    origin_size: usize,
    minified_size: Option<usize>,
    brotlied_size: usize,
}

// track file size among minify processes.
struct TrackedFile {
    content: Vec<u8>,
    path: PathBuf,
    original_len: usize,
}

impl TrackedFile {
    async fn new(path: impl Into<PathBuf>) -> Result<TrackedFile> {
        let path = path.into();
        let content = fs::read_file(&path).await?;
        let original_len = content.len();
        Ok(Self {
            content,
            path,
            original_len,
        })
    }

    async fn minify_str<F>(&mut self, minifier: F) -> Result<()>
    where
        F: FnOnce(String) -> Pin<Box<dyn Future<Output = Result<String>>>>,
    {
        let input = String::from_utf8(self.content.clone())?;
        let updated = minifier(input).await?;
        self.content = updated.into_bytes();
        Ok(())
    }

    async fn finish(self) -> Result<ProcessStats> {
        let maybe_minified_size = self.content.len();
        let brotlied_size = brotli::compress(&self.content).len();
        fs::write_file(
            &MINIFIED_DIR.join(self.path.file_name().unwrap()),
            &self.content,
        )
        .await?;
        Ok(ProcessStats {
            origin_size: self.original_len,
            minified_size: (self.original_len != maybe_minified_size)
                .then_some(maybe_minified_size),
            brotlied_size,
        })
    }
}

// Async Closure
macro_rules! ac {
    (|$i:ident$(:$ty:ty)?| $b:block) => {
        |$i$(:$ty)?| { Box::pin(async move { $b }) as Pin<Box<dyn Future<Output = _>>> }
    };
}

async fn start() -> Result<()> {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .with_timer(UtcTime::rfc_3339())
        .with_writer(tracing_web::MakeConsoleWriter)
        .with_span_events(FmtSpan::ACTIVE);
    let perf_layer = tracing_web::performance_layer().with_details_from_fields(Pretty::default());

    tracing_subscriber::registry()
        .with(SwcFilter)
        .with(fmt_layer)
        .with(perf_layer)
        .init();

    fs::rimraf(*MINIFIED_DIR).await?;
    fs::mkdir(*MINIFIED_DIR).await?;

    let mut file_paths = fs::read_dir(*ORIGINAL_DIR).await?;

    enum ProcessTarget {
        Individual(TrackedFile),
        WasmBindgen { js: TrackedFile, wasm: TrackedFile },
    }

    let mut js = vec![];
    let mut wasm = vec![];
    let mut targets = vec![];

    // grouping
    // html, css => Individual
    // js if wasm pair found => WasmBindgen { js, wasm }
    // other js and wasm => Individual
    while let Some(file) = file_paths.pop() {
        let Some(ext @ ("html" | "css" | "js" | "wasm")) = file.extension().and_then(|x| x.to_str()) else { continue };
        let file = TrackedFile::new(&file).await?;
        match ext {
            "html" | "css" => targets.push(ProcessTarget::Individual(file)),
            "wasm" => wasm.push(file),
            "js" => js.push(file),
            _ => unreachable!(),
        }
    }
    for js in js {
        let is_pair_wasm = |wasm: &TrackedFile| {
            wasm.path.file_stem().unwrap().to_str().unwrap()
                == js.path.file_stem().unwrap().to_str().unwrap().to_owned() + "_bg"
        };
        if let Some(idex) = wasm.iter().position(is_pair_wasm) {
            targets.push(ProcessTarget::WasmBindgen {
                js,
                wasm: wasm.remove(idex),
            });
        } else {
            targets.push(ProcessTarget::Individual(js));
        }
    }
    for wasm in wasm {
        targets.push(ProcessTarget::Individual(wasm));
    }

    // minify
    let minify_html = ac!(|x: String| { sys::minifier::html(&x).await });
    let minify_css = ac!(|x: String| { sys::minifier::css(&x).await });
    let minify_js = ac!(|x: String| { sys::minifier::js(&opt_js::optimize_js(x)).await });
    for target in &mut targets {
        match target {
            ProcessTarget::Individual(i) => match i.path.extension().unwrap().to_str().unwrap() {
                "html" => i.minify_str(&minify_html).await?,
                "css" => i.minify_str(&minify_css).await?,
                "js" => i.minify_str(&minify_js).await?,
                _ => {}
            },
            ProcessTarget::WasmBindgen { js, wasm } => {
                symbol::minify_symbol(&mut wasm.content, &mut js.content).await;
                js.minify_str(&minify_js).await?;
            }
        }
    }

    // finalize and show result
    let mut files = vec![];
    for target in targets {
        match target {
            ProcessTarget::Individual(i) => files.push(i),
            ProcessTarget::WasmBindgen { js, wasm } => {
                files.push(js);
                files.push(wasm);
            }
        }
    }

    let mut file_name_max_len = 0;
    for f in &files {
        file_name_max_len = file_name_max_len.max(
            f.path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .chars()
                .count(),
        );
    }

    println(format!(
        "{1:>0$}: {2:>10} {3:>10} {4:>10}",
        file_name_max_len, "filename", "origin", "minify", "brotli",
    ));

    for f in files {
        let file_name = f.path.file_name().unwrap().to_str().unwrap().to_owned();
        let stats = f.finish().await?;
        let kib = |n| format!("{:7.02}KiB", (n as f64) / 1024.0);
        println(format!(
            "{1:>0$}: {2:>} {3:>} {4:>}",
            file_name_max_len,
            file_name,
            kib(stats.origin_size),
            stats
                .minified_size
                .map_or_else(|| format!("{:>10}", "---KiB"), kib),
            kib(stats.brotlied_size),
        ))
    }

    Ok(())
}

// swc is too loud
struct SwcFilter;
impl<S: Subscriber> Layer<S> for SwcFilter {
    fn enabled(&self, metadata: &Metadata<'_>, _ctx: Context<'_, S>) -> bool {
        !matches!(metadata.module_path(), Some(mpath) if mpath.starts_with("swc"))
    }
}

fn println(s: String) {
    console::log_1(&JsValue::from(s));
}
