#[macro_export]
macro_rules! scenario_happ_path {
    ($name:literal) => {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../happs")
            .join(env!("CARGO_PKG_NAME"))
            .join(format!("{}.happ", $name))
            .canonicalize()
            .expect("Failed to canonicalize path to scenario hApp")
    };
}
