param(
    [string]$OutputPath = "dist/the-rust-programming-language.pdf"
)

$ErrorActionPreference = "Stop"

cargo run --release --bin book_pdf -- $OutputPath
