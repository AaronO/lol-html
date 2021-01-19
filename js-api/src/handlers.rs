use super::comment::Comment;
use super::doctype::Doctype;
use super::document_end::DocumentEnd;
use super::element::Element;
use super::text_chunk::TextChunk;
use super::*;
use js_sys::Function as JsFunction;
use lol_html::{
    DocumentContentHandlers as NativeDocumentContentHandlers,
    ElementContentHandlers as NativeElementContentHandlers, Selector,
};
use std::borrow::Cow;
use std::mem;
use thiserror::Error;

// NOTE: Display is noop, because we'll unwrap JSValue error when it will be propagated to
// `write()` or `end()`.
#[derive(Error, Debug)]
#[error("JS handler error")]
pub struct HandlerJsErrorWrap(pub String);

macro_rules! make_handler {
    ($handler:ident, $JsArgType:ident) => {
        move |arg: &mut _| {
            let (js_arg, anchor) = $JsArgType::from_native(arg);
            let this = JsValue::NULL;
            let js_arg = JsValue::from(js_arg);

            let res = match $handler.call1(&this, &js_arg) {
                Ok(_) => Ok(()),
                Err(e) => Err(HandlerJsErrorWrap(e.as_string().unwrap()).into()),
            };

            mem::drop(anchor);

            res
        }
    };
}

macro_rules! add_handler {
    ($builder: ident, $jsObj:ident, $method:ident, $JsArgType:ident) => {
        if let Some(handler) = $jsObj.$method() {
            $builder = $builder.$method(make_handler!(handler, $JsArgType));
        }
    };
}

#[wasm_bindgen]
extern "C" {
    pub type ElementContentHandlers;

    #[wasm_bindgen(method, getter)]
    fn element(this: &ElementContentHandlers) -> Option<JsFunction>;
    #[wasm_bindgen(method, getter)]
    fn comments(this: &ElementContentHandlers) -> Option<JsFunction>;
    #[wasm_bindgen(method, getter)]
    fn text(this: &ElementContentHandlers) -> Option<JsFunction>;
}

impl IntoNative<NativeElementContentHandlers<'static>> for ElementContentHandlers {
    fn into_native(self) -> NativeElementContentHandlers<'static> {
        let mut native = NativeElementContentHandlers::default();

        add_handler!(native, self, element, Element);
        add_handler!(native, self, comments, Comment);
        add_handler!(native, self, text, TextChunk);

        native
    }
}

#[wasm_bindgen]
extern "C" {
    pub type DocumentContentHandlers;

    #[wasm_bindgen(method, getter)]
    fn doctype(this: &DocumentContentHandlers) -> Option<JsFunction>;
    #[wasm_bindgen(method, getter)]
    fn comments(this: &DocumentContentHandlers) -> Option<JsFunction>;
    #[wasm_bindgen(method, getter)]
    fn text(this: &DocumentContentHandlers) -> Option<JsFunction>;
    #[wasm_bindgen(method, getter)]
    fn end(this: &DocumentContentHandlers) -> Option<JsFunction>;
}

impl IntoNative<NativeDocumentContentHandlers<'static>> for DocumentContentHandlers {
    fn into_native(self) -> NativeDocumentContentHandlers<'static> {
        let mut native = NativeDocumentContentHandlers::default();

        add_handler!(native, self, doctype, Doctype);
        add_handler!(native, self, comments, Comment);
        add_handler!(native, self, text, TextChunk);
        add_handler!(native, self, end, DocumentEnd);

        native
    }
}

// Element handlers
#[derive(Clone)]
pub(crate) struct VecElHandlers(pub(crate) Vec<(String, JsValue)>);
impl VecElHandlers {
    pub fn into_native<'a>(self) -> Vec<(Cow<'a, Selector>, NativeElementContentHandlers<'a>)> {
        self.0
            .into_iter()
            .map(|(selector, h)| {
                (
                    Cow::Owned(selector.parse().unwrap()),
                    ElementContentHandlers::from(h).into_native(),
                )
            })
            .collect()
    }
}

// Doc handlers
#[derive(Clone)]
pub(crate) struct VecDocHandlers(pub(crate) Vec<JsValue>);
impl VecDocHandlers {
    pub fn into_native<'a>(self) -> Vec<NativeDocumentContentHandlers<'a>> {
        self.0
            .into_iter()
            .map(|h| DocumentContentHandlers::from(h).into_native())
            .collect()
    }
}
