use super::*;
use js_sys::{Function as JsFunction, Reflect};
use lol_html::{DocumentContentHandlers, ElementContentHandlers, Selector};
use std::borrow::Cow;

// JsHandlers represents a collection of JS callbacks
#[derive(Clone)]
struct JsHandlers(JsValue);
impl JsHandlers {
    fn new(obj: JsValue) -> Self {
        JsHandlers(obj.clone())
    }

    fn method(&mut self, method: &str) -> JsFunction {
        Reflect::get(&self.0, &JsValue::from_str(method))
            .map(|v| JsFunction::from(v))
            .unwrap()
    }

    fn has(&mut self, method: &str) -> bool {
        Reflect::has(&self.0, &JsValue::from_str(method)).unwrap()
    }
}

// A macro to reduce verbosity in building conditional handlers
macro_rules! build_handler {
    ($h: ident, $hx: ident, $method_name:literal, $builder_name:ident, $ty:ty) => {
        if $h.has($method_name) {
            let m = $h.method($method_name);
            $hx = $hx.$builder_name(move |x| {
                m.call1(&JsValue::NULL, &<$ty>::from_native(x).0.into())
                    .unwrap();
                Ok(())
            });
        }
    };
}

// Element handlers
pub(crate) struct JsElHandlers(pub(crate) Vec<(String, JsValue)>);
type NativeElHandlers<'a> = Vec<(Cow<'a, Selector>, ElementContentHandlers<'a>)>;
impl JsElHandlers {
    pub fn into_native<'a>(&self) -> NativeElHandlers<'a> {
        self.0
            .iter()
            .map(|(selector, h)| {
                (
                    Cow::Owned(selector.parse().unwrap()),
                    Self::single_native((*h).clone()),
                )
            })
            .collect()
    }

    fn single_native<'a>(obj: JsValue) -> ElementContentHandlers<'a> {
        let mut h = JsHandlers::new(obj);
        let mut hx = ElementContentHandlers::default();

        build_handler!(h, hx, "element", element, element::Element);
        build_handler!(h, hx, "comments", comments, comment::Comment);
        build_handler!(h, hx, "text", text, text_chunk::TextChunk);

        hx
    }
}

// Doc handlers
pub(crate) struct JsDocHandlers(pub(crate) Vec<JsValue>);
type NativeDocHandlers<'a> = Vec<DocumentContentHandlers<'a>>;
impl JsDocHandlers {
    pub fn into_native<'a>(&self) -> NativeDocHandlers<'a> {
        self.0
            .iter()
            .map(|h| (Self::single_native((*h).clone())))
            .collect()
    }

    fn single_native<'a>(obj: JsValue) -> DocumentContentHandlers<'a> {
        let mut h = JsHandlers::new(obj);
        let mut hx = DocumentContentHandlers::default();

        build_handler!(h, hx, "comments", comments, comment::Comment);
        build_handler!(h, hx, "text", text, text_chunk::TextChunk);
        build_handler!(h, hx, "end", end, document_end::DocumentEnd);
        build_handler!(h, hx, "doctype", doctype, doctype::Doctype);

        hx
    }
}

// #[wasm_bindgen]
// extern "C" {
//     pub type ExternElementHandlers;

//     #[wasm_bindgen(structural, method)]
//     pub fn element(this: &ExternElementHandlers, el: Element);
//     #[wasm_bindgen(structural, method)]
//     pub fn text(this: &ExternElementHandlers, text: TextChunk);
//     #[wasm_bindgen(structural, method)]
//     pub fn comments(this: &ExternElementHandlers, comment: Comment);
// }

// #[wasm_bindgen]
// extern "C" {
//     pub type ExternDocumentHandlers;

//     #[wasm_bindgen(structural, method)]
//     pub fn doctype(this: &ExternDocumentHandlers, doctype: doctype::Doctype);
//     #[wasm_bindgen(structural, method)]
//     pub fn text(this: &ExternDocumentHandlers, text: text_chunk::TextChunk);
//     #[wasm_bindgen(structural, method)]
//     pub fn comments(this: &ExternDocumentHandlers, comment: comment::Comment);
//     #[wasm_bindgen(structural, method)]
//     pub fn end(this: &ExternDocumentHandlers, end: document_end::DocumentEnd);
// }
