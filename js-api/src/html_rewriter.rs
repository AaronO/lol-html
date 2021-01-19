use super::handlers::{
    DocumentContentHandlers, ElementContentHandlers, HandlerJsErrorWrap, VecDocHandlers,
    VecElHandlers,
};
use super::*;
use js_sys::{Function as JsFunction, Uint8Array};
use lol_html::errors::RewritingError;
use lol_html::{
    rewrite_str, HtmlRewriter as NativeHTMLRewriter, OutputSink, RewriteStrSettings, Settings,
};

#[derive(Clone)]
pub struct JsOutputSink(JsFunction);

impl JsOutputSink {
    fn new(func: JsFunction) -> Self {
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

fn rewriting_error_to_js(err: RewritingError) -> JsValue {
    match err {
        RewritingError::ContentHandlerError(err) => {
            JsValue::from(err.downcast::<HandlerJsErrorWrap>().unwrap().0)
        }
        _ => JsValue::from(err.to_string()),
    }
}

#[wasm_bindgen]
pub struct HTMLRewriter {
    el_handlers: VecElHandlers,
    doc_handlers: VecDocHandlers,
}

#[wasm_bindgen]
impl HTMLRewriter {
    #[wasm_bindgen(constructor)]
    pub fn new() -> HTMLRewriter {
        HTMLRewriter {
            el_handlers: VecElHandlers(vec![]),
            doc_handlers: VecDocHandlers(vec![]),
        }
    }

    pub fn on(&mut self, selector: &str, h: ElementContentHandlers) {
        self.el_handlers.0.push((String::from(selector), h.into()));
    }

    #[wasm_bindgen(method, js_name=onDocument)]
    pub fn on_document(&mut self, h: DocumentContentHandlers) {
        self.doc_handlers.0.push(h.into());
    }

    #[wasm_bindgen(method, js_name=transformString)]
    pub fn transform_string(&mut self, input: String) -> JsResult<String> {
        rewrite_str(
            &input,
            RewriteStrSettings {
                document_content_handlers: self.doc_handlers.clone().into_native(),
                element_content_handlers: self.el_handlers.clone().into_native(),
                ..RewriteStrSettings::default()
            },
        )
        .map_err(rewriting_error_to_js)
    }

    #[wasm_bindgen(method)]
    pub fn transform(self, input: JsValue) -> JsResult<JsValue> {
        Ok(rewrite(self, input))
    }

    #[wasm_bindgen(method, js_name=newStream)]
    pub fn new_stream(&mut self, js_sink: JsFunction) -> RewriteStream {
        RewriteStream::new(self, js_sink)
    }
}

#[wasm_bindgen(module = "/js/rewriter.js")]
extern "C" {
    fn rewrite(builder: HTMLRewriter, obj: JsValue) -> JsValue;
}

#[wasm_bindgen]
pub struct RewriteStream {
    inner: NativeHTMLRewriter<'static, JsOutputSink>,
}

#[wasm_bindgen]
impl RewriteStream {
    #[wasm_bindgen(constructor)]
    pub fn new(builder: &HTMLRewriter, js_sink: JsFunction) -> RewriteStream {
        let rewriter = NativeHTMLRewriter::new(
            Settings {
                document_content_handlers: builder.doc_handlers.clone().into_native(),
                element_content_handlers: builder.el_handlers.clone().into_native(),
                ..Settings::default()
            },
            JsOutputSink::new(js_sink),
        );

        RewriteStream { inner: rewriter }
    }

    pub fn write(&mut self, chunk: &[u8]) -> JsResult<()> {
        self.inner.write(chunk).map_err(rewriting_error_to_js)
    }

    pub fn end(self) -> JsResult<()> {
        self.inner.end().map_err(rewriting_error_to_js)
        // Ok(())
    }
}
