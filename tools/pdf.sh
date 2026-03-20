#!/bin/bash

set -eu

cargo run --release --bin book_pdf -- "${1:-dist/the-rust-programming-language.pdf}"
