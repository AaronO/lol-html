var fs = require('fs');

console.time('setup');
var r = require('./pkg/lol_html_js_api');
console.timeEnd('setup');
var hb = new r.HtmlRewriterBuilder();

hb.onDocument({
    comments: (c) => c.text,
    // text: (t) => t.replace(t.text+'.'),
    // end: () => console.log('end')
})

setTimeout(
    () => {
        const src = fs.readFileSync('./test.html').toString();
        console.time('transform');
        // hb.transformString(`<div id="foo"><!-- special comment 🌈 --></div><img>`)
        for(let i = 0; i < 100; i++) {
            // console.log('ptr:', hb.ptr);
            hb.transformString(src);
        }
        console.timeEnd('transform');
        fs.writeFileSync('./patched.html', hb.transformString(src));
        console.log(src.length, 'bytes');
    },
    0
);
