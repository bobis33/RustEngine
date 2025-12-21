<p align="center">
  <a href="https://github.com/bobis33/vers/blob/main/assets/icons/icon.png">
    <img src="assets/icons/icon.png" alt="Logo" width="200" height="200">
  </a>

  <h1 align="center">VERS</h1>

  <p align="center">
    Vulkan based engine.
  </p>

<p align="center">
  <a href="https://rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.93.0-orange" alt="Rust 1.93.0"/></a>
  <a href="https://github.com/bobis33/vers/blob/main/LICENSE.md"><img src="https://img.shields.io/badge/license-MIT-blue" alt="MIT License"/></a>
  <a href="https://github.com/bobis33/vers/actions/workflows/ci.yml"><img src="https://github.com/bobis33/vers/actions/workflows/ci.yml/badge.svg" alt="CI build Status"/></a>
  <a href="https://github.com/bobis33/vers/actions/workflows/deploy-documentation.yml"><img src="https://github.com/bobis33/vers/actions/workflows/deploy-documentation.yml/badge.svg" alt="CD deploy documentation Status"/></a>
</p>

---

# Platforms

| Platform | Status |
|:---------|:-------|
| Windows  | ✅      |
| Linux    | ✅      |
| MacOS    | ❌      |
| Web      | ❌      |
| Android  | ❌      |
| iOS      | ❌      |
| tvOS     | ❌      |

> ✅ = working, 🚧 = in progress, ❌ = not handled

## 🧩 Project Structure

```text
crates/
├── vers-asset/       → lib  (asset loading, hot-reload)
├── vers-audio/       → lib  (audio engine & system)
├── vers-core/        → lib  (math, fundamental types, ECS)
├── vers-engine/      → lib  (facade crate, re-exports all modules)
├── vers-platform/    → lib  (windowing, input, OS abstraction)
├── vers-render/      → lib  (Vulkan pipelines, materials, renderer)
└── vers-tools/       → lib  (tools) 
```

# Prerequisites
- Rust 1.93+ ([install](https://rustup.rs/))
- Vulkan SDK ([install](https://vulkan.lunarg.com/sdk/home))

# Usage
Run examples: <br>
`cargo run --example [ example_to_run ]`
> run `cargo run --example` to list available examples

Run tests: <br>
`cargo test`

Generate docs: <br>
`cargo doc`

Run benchmarks: <br>
`cargo bench`

Lint project: <br>
`cargo clippy`

Format project: <br>
`cargo fmt`

## Security
See the [security policy](https://github.com/bobis33/vers/blob/main/SECURITY.md) for more information.

## Contributing
Want to contribute? See [contributing guidelines](https://github.com/bobis33/vers/blob/main/CONTRIBUTING.md).

Please read our [code of conduct](https://github.com/bobis33/vers/blob/main/CODE_OF_CONDUCT.md) before contributing to this project.

## License
This project is licensed under the **MIT license**. See the [license](https://github.com/bobis33/vers/blob/main/LICENSE) file for details.
