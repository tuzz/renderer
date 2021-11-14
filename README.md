## WebGPU Renderer

A Rust crate that abstracts over the [wgpu](https://github.com/gfx-rs/wgpu)
crate to provide greatly simplified rendering functionality for a modern
vertex/fragment shader pipeline. The [quads example](./examples/quads.rs) shows
how to use the crate. You can run it with:

```sh
$ cargo run --release --example quads --all-features
```

This crate is based on the first part of
[this](https://sotrh.github.io/learn-wgpu) excellent tutorial and tries to
neatly organise all the moving pieces and provide a straightforward interface.
The cost of this simplification is flexibility. If you need anything more than
a conventional vertex/fragment shader pipeline (with textures) then you'll want
something else.

Currently this crate has no documentation and isn't published so you'll need to
clone it or depend on it from GitHub if you want to use it. For example:

```tomml
# Cargo.toml
[dependencies]
renderer = { git = "https://github.com/tuzz/renderer }
```

## License

MIT
