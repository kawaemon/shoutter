pub mod brotli;
pub mod fs;
pub mod minifier;
use wasm_bindgen::JsValue;

#[derive(Debug)]
struct JsError(JsValue);

unsafe impl Send for JsError {}
unsafe impl Sync for JsError {}

impl std::fmt::Display for JsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as std::fmt::Debug>::fmt(self, f)
    }
}
impl std::error::Error for JsError {}

macro_rules! object {
    ($($key:ident: $value:expr),*$(,)?) => {{
        let obj = js_sys::Object::new();
        $(js_sys::Reflect::set(&obj, &JsValue::from(stringify!($key)), &JsValue::from($value))
            .expect("setting property on the object should never fail.");)*
        obj
    }};
}

use object;
