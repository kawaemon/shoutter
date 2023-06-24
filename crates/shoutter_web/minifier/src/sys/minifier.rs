use anyhow::Result;
use js_sys::{Array, Function, Object, Reflect};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};

use crate::sys::{object, JsError};

fn html_minifier_option() -> Object {
    object! {
        collapseBooleanAttributes: true,
        collapseWhitespace: true,
        decodeEntities: true,
        html5: true,
        minifyCSS: true,
        minifyJS: js_minifier_option(),
        processConditionalComments: true,
        removeAttributeQuotes: true,
        removeComments: true,
        removeEmptyAttributes: true,
        removeOptionalTags: true,
        removeRedundantAttributes: true,
        removeScriptTypeAttributes: true,
        removeStyleLinkTypeAttributes: true,
        removeTagWhitespace: true,
        sortAttributes: true,
        sortClassName: true,
        trimCustomFragments: true,
        useShortDoctype: true,
    }
}

fn js_minifier_option() -> Object {
    object! {
        ecma: 2021,
        toplevel: true,
        compress: object! {
            ecma: 2021,
            passes: 3,
            pure_getters: true,
        },
    }
}

pub async fn html(html: &str) -> Result<String> {
    #[wasm_bindgen(module = "html-minifier-terser")]
    extern "C" {
        #[wasm_bindgen(catch)]
        async fn minify(html: &str, options: Object) -> Result<JsValue, JsValue>;
    }

    let res = minify(html, html_minifier_option())
        .await
        .map_err(JsError)?
        .as_string()
        .expect("html minifier should return string");

    Ok(res)
}

pub async fn css(css: &str) -> Result<String> {
    // there is no way currently to deal with default exports.
    #[wasm_bindgen]
    extern "C" {
        fn require(s: &str) -> Object;
    }

    let clean_css = require("clean-css")
        .dyn_into::<Function>()
        .expect("require('clean-css') should return function");
    let clean_css = Reflect::construct(&clean_css, &Array::new()).expect("new CleanCss() failed");
    let minifier = Reflect::get(&clean_css, &JsValue::from("minify"))
        .expect("clean-css instance should have minify method")
        .dyn_into::<Function>()
        .expect("method should be function");
    let minified =
        Reflect::apply(&minifier, &clean_css, &Array::of1(&JsValue::from(css))).map_err(JsError)?;
    let minified = Reflect::get(&minified, &JsValue::from("styles"))
        .expect("minifiy response should have styles key")
        .as_string()
        .expect("minifyResponse.styles should be string");

    Ok(minified)
}

pub async fn js(js: &str) -> Result<String> {
    #[wasm_bindgen(module = "terser")]
    extern "C" {
        #[wasm_bindgen(catch)]
        async fn minify(js: &str, option: Object) -> Result<JsValue, JsValue>;
    }

    let res = minify(js, js_minifier_option()).await.map_err(JsError)?;
    let res = Reflect::get(&res, &JsValue::from("code"))
        .expect("minify response should have `code` key")
        .as_string()
        .expect("minifyResponse.code should be string");

    Ok(res)
}
