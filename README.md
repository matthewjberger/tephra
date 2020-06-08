# Tephra

Demonstrations of various graphics programming techniques using Vulkan and Rust.

# Instructions

```powershell
cargo run --release --bin teapot
cargo run --release --bin pbr


# To enable loading validation layers:

# Run in debug mode.
# Validation layers are loaded by default in debug mode.
cargo run --bin pbr

# Or use release mode and enable the 'validation' feature flag
cargo run --bin pbr --release --features validation
```
