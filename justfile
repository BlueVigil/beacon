set dotenv-load

cargo := "rustup run nightly cargo"
rustc := `rustup which --toolchain nightly rustc`

default:
    @just --list

help:
    @just --list

format:
    {{cargo}} fmt --all

format-check:
    {{cargo}} fmt --all -- --check

lint:
    RUSTC={{rustc}} {{cargo}} check
    RUSTC={{rustc}} {{cargo}} clippy --all-targets --all-features -- -D warnings

clean:
    {{cargo}} clean

run:
    RUSTC={{rustc}} {{cargo}} run

package target:
    ./scripts/package.sh {{target}}

package-macos-aarch64:
    just package aarch64-apple-darwin

package-macos-x86_64:
    just package x86_64-apple-darwin

package-linux-aarch64:
    just package aarch64-unknown-linux-gnu

package-linux-x86_64:
    just package x86_64-unknown-linux-gnu

package-windows-aarch64:
    just package aarch64-pc-windows-msvc

package-windows-x86_64:
    just package x86_64-pc-windows-gnu

package-all:
    just package-macos-aarch64
    just package-macos-x86_64
    just package-linux-aarch64
    just package-linux-x86_64
    just package-windows-aarch64
    just package-windows-x86_64
