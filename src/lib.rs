extern crate lazycell;
extern crate safemem;

pub mod base;
pub mod errors;
pub mod tokenizer;
pub mod transform_stream;

// TODO
// -- Functionality
// 3. Eager tokenizer
// 4. Tokenizer driver
// 5. Adjustable limits
// 6. Get rid of token view as we don't need to store buffer anymore
//
// -- Performance
// 1. Implement benchmark
// 2. Get rid of dynamic dispatch for input (chunk from buffer)
// 3. LTO
// 4. In-state loops
// 5. Don't emit character immidiately, extend existing
// 6. State embedding
