set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

default:
    @just --list

setup:
    lefthook install

format:
    cargo fmt --all
    cargo fmt --manifest-path gallery/Cargo.toml

format-check:
    cargo fmt --all -- --check
    cargo fmt --manifest-path gallery/Cargo.toml -- --check

# The library for the host, the web (wasm32) and Android, and the gallery for
# each of its builds: web client, server, and mobile. The gallery's mobile
# build is checked on the host, as g3-stack does: for the Android target its
# TLS stack (ring) needs the NDK's C compiler.
check: && check-ios
    cargo check
    cargo check --target wasm32-unknown-unknown
    cargo check --target aarch64-linux-android
    cargo check --manifest-path gallery/Cargo.toml --features web --target wasm32-unknown-unknown
    cargo check --manifest-path gallery/Cargo.toml --no-default-features --features server
    cargo check --manifest-path gallery/Cargo.toml --no-default-features --features mobile

# Dioxus's asset crate pulls in objc2 on iOS, whose build script compiles C
# through Xcode, so only a Mac can build that target.
[macos]
check-ios:
    cargo check --target aarch64-apple-ios

[linux]
[windows]
check-ios:
    @echo "Skipping aarch64-apple-ios: it needs Xcode."

lint:
    cargo clippy --all-targets --no-deps
    cargo clippy --no-deps --target wasm32-unknown-unknown
    cargo clippy --manifest-path gallery/Cargo.toml --no-default-features --features server --all-targets --no-deps

lint-strict:
    cargo clippy --all-targets --no-deps -- -D warnings
    cargo clippy --no-deps --target wasm32-unknown-unknown -- -D warnings
    cargo clippy --no-deps --target aarch64-linux-android -- -D warnings
    cargo clippy --manifest-path gallery/Cargo.toml --no-default-features --features server --all-targets --no-deps -- -D warnings
    cargo clippy --manifest-path gallery/Cargo.toml --features web --target wasm32-unknown-unknown --no-deps -- -D warnings
    cargo clippy --manifest-path gallery/Cargo.toml --no-default-features --features mobile --no-deps -- -D warnings

test:
    cargo nextest run
    cargo test --doc

spell:
    typos

doc:
    cargo doc --no-deps --config "build.rustdocflags='-D warnings'"

security:
    cargo deny check

pre-push: format-check check lint-strict test spell

quality: pre-push

package-check:
    cargo package --allow-dirty --list

ci: quality doc security package-check

# The gallery: every chart, light and dark, md and ios, at http://127.0.0.1:8095.
gallery:
    cd gallery; dx serve --web --addr 127.0.0.1 --port 8095 --open false

# Readability rules inside RSX, checked without rewriting source.
style-check:
    node scripts/rust-style.mjs

style-test:
    node --test scripts/rust-style.test.mjs
