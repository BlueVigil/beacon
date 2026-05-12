set dotenv-load

export RUSTC := "rustc"

cargo := "cargo"
nightly-cargo := "rustup run nightly cargo"

default:
    @just --list

help:
    @just --list

format:
    {{nightly-cargo}} fmt --all

format-check:
    {{nightly-cargo}} fmt --all -- --check

lint:
    {{cargo}} clippy --all-targets --all-features -- -D warnings

compile:
    {{cargo}} check

clean:
    {{cargo}} clean

build:
    {{cargo}} build

run:
    {{cargo}} run
