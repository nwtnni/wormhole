const ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn main() {
    let files = ["lib.c", "kv.c", "wh.c"]
        .into_iter()
        .map(|path| format!("{ROOT}/../{path}"))
        .collect::<Vec<_>>();

    let include = format!("{ROOT}/../");

    cc::Build::new()
        .files(&files)
        .include(&include)
        .flag("-march=native")
        .compile("wormhole");

    for path in files.iter().chain([&include]) {
        println!("cargo:rerun-if-changed={path}");
    }
}
