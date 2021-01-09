use super::*;
use js_sys::{Function as JsFunction, Uint8Array};
use lol_html::{
    rewrite_str, DocumentContentHandlers, ElementContentHandlers,
    HtmlRewriter as NativeHtmlRewriter, OutputSink, RewriteStrSettings, Selector, Settings,
};
use web_sys;

struct JsOutputSink(JsFunction);

impl JsOutputSink {
    fn new(func: &JsFunction) -> Self {
        JsOutputSink(func.clone())
    }
}

impl OutputSink for JsOutputSink {
    #[inline]
    fn handle_chunk(&mut self, chunk: &[u8]) {
        let this = JsValue::NULL;
        let chunk = Uint8Array::from(chunk);

        // NOTE: the error is handled in the JS wrapper.
        self.0.call1(&this, &chunk).unwrap();
    }
}

#[wasm_bindgen]
pub struct HtmlRewriterBuilder {
    el_handlers: handlers::JsElHandlers,
    doc_handlers: handlers::JsDocHandlers,
}

#[wasm_bindgen]
impl HtmlRewriterBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> HtmlRewriterBuilder {
        HtmlRewriterBuilder {
            el_handlers: handlers::JsElHandlers(vec![]),
            doc_handlers: handlers::JsDocHandlers(vec![]),
        }
    }

    pub fn on(&mut self, selector: &str, h: JsValue) {
        self.el_handlers.0.push((String::from(selector), h));
    }

    #[wasm_bindgen(method, js_name=onDocument)]
    pub fn on_document(&mut self, h: JsValue) {
        self.doc_handlers.0.push(h);
    }

    // transform handles Response/ReadableStream/String
    pub fn transform(&mut self, value: JsValue) -> JsResult<JsValue> {
        let g = js_sys::global();

        Ok(JsValue::NULL)
    }

    #[wasm_bindgen(method, js_name=transformResponse)]
    pub fn transform_response(&mut self, orig: web_sys::Response) -> JsResult<web_sys::Response> {
        // let r = NativeHtmlRewriter::new(Settings {
        //     document_content_handlers: self.document_content_handlers,
        //     element_content_handlers: self.element_content_handlers,
        //     ..Settings::default()
        // }, JsOutputSink::new(js_sink));

        // Create new body stream
        let new_body = self.transform_stream(orig.body().unwrap());

        // Copy headers & status-code
        let mut init = web_sys::ResponseInit::new();
        init.headers(&orig.headers());
        init.status(orig.status());

        // Return new response with body transformed by HtmlRewriter
        web_sys::Response::new_with_opt_readable_stream_and_init(Some(&new_body), &init)
    }

    #[wasm_bindgen(method, js_name=transformStream)]
    pub fn transform_stream(&mut self, input: web_sys::ReadableStream) -> web_sys::ReadableStream {
        input
    }

    #[wasm_bindgen(method, js_name=transformString)]
    pub fn transform_string(&mut self, input: String) -> String {
        rewrite_str(
            &input,
            RewriteStrSettings {
                document_content_handlers: self.doc_handlers.into_native(),
                element_content_handlers: self.el_handlers.into_native(),
                ..RewriteStrSettings::default()
            },
        )
        .unwrap()
    }
}

#[wasm_bindgen(module = "/js/BadBitch.js")]
extern "C" {
    type BadBitch;

    // This is a method on the JavaScript "String" class, so specify that with
    // the `js_class` attribute.
    #[wasm_bindgen(method, js_class = "BadBitch", js_name = hello)]
    fn hello(this: &BadBitch);
}

#[wasm_bindgen]
pub struct HtmlRewriter(NativeRefWrap<NativeHtmlRewriter<'static, JsOutputSink>>);

// impl_from_native!(NativeHtmlRewriter<'static, JsOutputSink> --> HtmlRewriter);

// impl HtmlRewriter {
//     pub fn from_native(inner: &'static mut NativeHtmlRewriter<'static, JsOutputSink>) -> (Self, Anchor<'static>) {
//         let (ref_wrap, anchor) = NativeRefWrap::wrap(inner);
//         (HtmlRewriter(ref_wrap), anchor)
//     }
// }

// #[wasm_bindgen]
// impl HtmlRewriter {
//     pub fn write(&mut self, chunk: Uint8Array) {
//         let vec = chunk.to_vec();
//         let c: &[u8] = &vec;
//         self.0.get_mut().map(|x| x.write(c));
//     }

//     pub fn end(&mut self) {
//         // self.0.get_mut().map(|x| x.end());
//     }
// }
