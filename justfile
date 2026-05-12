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
    RUSTC={{rustc}} {{cargo}} clippy --all-targets --all-features -- -D warnings

compile:
    RUSTC={{rustc}} {{cargo}} check

clean:
    {{cargo}} clean

clean-toolchain:
    {{cargo}} clean

build:
    RUSTC={{rustc}} {{cargo}} build

run:
    RUSTC={{rustc}} {{cargo}} run
