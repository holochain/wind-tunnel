#[macro_export]
macro_rules! embed_conductor_config {
    () => {
        fn conductor_config() -> &'static str {
            static CONDUCTOR_CONFIG: &str = include_str!("../../../conductor-config.yaml");
            static CONDUCTOR_CONFIG_CI: &str = include_str!("../../../conductor-config-ci.yaml");

            match std::env::var("CONDUCTOR_CONFIG") {
                Ok(value) if value == "CI" => CONDUCTOR_CONFIG_CI,
                _ => CONDUCTOR_CONFIG,
            }
        }
    };
}
