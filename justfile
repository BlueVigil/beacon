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

bundle-mac-aarch64:
    ./scripts/bundle-mac aarch64-apple-darwin

bundle-mac-x86_64:
    ./scripts/bundle-mac x86_64-apple-darwin

bundle-linux-aarch64:
    ./scripts/bundle-linux aarch64-unknown-linux-gnu

bundle-linux-x86_64:
    ./scripts/bundle-linux x86_64-unknown-linux-gnu

bundle-windows-x86_64:
    pwsh ./scripts/bundle-windows.ps1 -Architecture x86_64

bundle-windows-aarch64:
    pwsh ./scripts/bundle-windows.ps1 -Architecture aarch64

bundle-all:
    just bundle-mac-aarch64
    just bundle-mac-x86_64
    just bundle-linux-aarch64
    just bundle-linux-x86_64
    just bundle-windows-x86_64
    just bundle-windows-aarch64

icon size="1024":
    ./scripts/render-icon.sh {{size}}
