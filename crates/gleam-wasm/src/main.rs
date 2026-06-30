fn main() {
    gleam_wasm::run().unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    });
}
