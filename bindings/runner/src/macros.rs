#[macro_export]
macro_rules! happ_path {
    ($name:literal) => {{
        let local_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../happs")
            .join(env!("CARGO_PKG_NAME"))
            .join(format!("{}.happ", $name));

        if let Ok(path) = local_path.canonicalize() {
            path
        } else {
            // Looking for a Nix store path, which will look something like `/nix/store/xxx/bin/../happs/$name.happ`
            let nix_path = std::env::current_exe()
                .expect("Could not get current executable path")
                .parent()
                .and_then(|p| p.parent())
                .map(|p| p.join("happs").join(format!("{}.happ", $name)));

            if let Some(nix_path) = nix_path.as_ref().and_then(|p| p.canonicalize().ok()) {
                nix_path
            } else {
                panic!("Could not find the happ at either the local path {local_path:?} or the nix path {nix_path:?}");
            }
        }
    }};
}
