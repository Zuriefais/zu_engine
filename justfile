run:
    RUST_LOG=info cargo run --package zu_core

run_release:
    RUST_LOG=info cargo run --package zu_core --release


build:
    cargo build --release --package zu_core
