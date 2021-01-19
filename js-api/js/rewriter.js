export function rewrite(builder, obj) {
    // String
    if(obj instanceof String) {
        return builder.transformString(obj);
    }
    // Stream
    if(obj && obj.pipeTo instanceof Function) {
        return rewriteStream(builder, obj);
    }

    // Response
    if(obj && obj.body !== undefined) {
        return rewriteResponse(builder, obj);
    }

    // TODO: error
}

function rewriteResponse(builder, resp) {
    return new Response(resp.body ? rewriteStream(builder, resp.body) : resp.body, {
        status: resp.status,
        headers: resp.headers,
    });
}

function rewriteStream(builder, stream) {
    let _controller;
    const rewriter = builder.newStream((chunk) => {
        if(!chunk || chunk.length === 0) {
            _controller.terminate();
            return;
        }
        const copyChunk = Uint8Array.from(chunk);
        _controller.enqueue(copyChunk);
    })
    const { readable, writable } = new TransformStream({
        start: (controller) => {
            _controller = controller;
        },
        transform: (chunk) => {
            rewriter.write(chunk);
        },
        flush: () => {
            rewriter.end();
        },
    });

    // Pipe and "swallow" errors
    // to avoid Deno failing with dangling rejected promises
    // the error is not ignored and is passed on through the stream
    // so this avoids "duplicating" the error
    stream.pipeTo(writable).catch(() => {});

    return readable;
}
