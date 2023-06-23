use std::path::{Path, PathBuf};

use anyhow::Result;
use js_sys::{Array, Object, Uint8Array};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};

use crate::sys::{object, JsError};

pub async fn rimraf(path: &Path) -> Result<()> {
    #[wasm_bindgen(module = "fs/promises")]
    extern "C" {
        #[wasm_bindgen(catch)]
        async fn rm(path: &str, options: &Object) -> Result<(), JsValue>;
    }

    rm(
        path.to_str().unwrap(),
        &object! {
            recursive: true,
            force: true,
        },
    )
    .await
    .map_err(JsError)?;

    Ok(())
}

pub async fn mkdir(path: &Path) -> Result<()> {
    #[wasm_bindgen(module = "fs/promises")]
    extern "C" {
        #[wasm_bindgen(catch)]
        async fn mkdir(path: &str) -> Result<(), JsValue>;
    }

    mkdir(path.to_str().unwrap()).await.map_err(JsError)?;
    Ok(())
}

pub async fn read_file(path: &Path) -> Result<Vec<u8>> {
    #[wasm_bindgen(module = "fs/promises")]
    extern "C" {
        #[wasm_bindgen(js_name = readFile, catch)]
        async fn read_file(path: &str) -> Result<JsValue, JsValue>;
    }

    let bytes = read_file(path.to_str().unwrap()).await.map_err(JsError)?;
    let bytes = bytes
        .dyn_into::<Uint8Array>()
        .expect("Buffer should be instance of Uint8Array");

    Ok(bytes.to_vec())
}

pub async fn write_file(path: &Path, data: &[u8]) -> Result<()> {
    #[wasm_bindgen(module = "fs/promises")]
    extern "C" {
        #[wasm_bindgen(js_name = writeFile, catch)]
        async fn write_file(path: &str, data: Uint8Array) -> Result<(), JsValue>;
    }

    let bytes = Uint8Array::from(data);
    write_file(path.to_str().unwrap(), bytes)
        .await
        .map_err(JsError)?;

    Ok(())
}

pub async fn read_dir(dir: &Path) -> Result<Vec<PathBuf>> {
    #[wasm_bindgen(module = "fs/promises")]
    extern "C" {
        #[wasm_bindgen(catch)]
        async fn readdir(path: &str) -> Result<JsValue, JsValue>;
    }

    let object = readdir(dir.to_str().unwrap()).await.map_err(JsError)?;
    let array = Array::from(&object);

    let mut ret = Vec::with_capacity(array.length() as _);
    for i in 0..array.length() {
        let file = array
            .get(i)
            .as_string()
            .expect("returned array should contain only strings");
        ret.push(PathBuf::from(dir).join(file));
    }

    Ok(ret)
}
