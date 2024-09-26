#[macro_export]
macro_rules! embed_conductor_config {
    () => {
        fn conductor_config() -> String {
            static BASE_CONDUCTOR_CONFIG: &str =
                include_str!("../../../conductor-config/conductor-config.yaml");
            static BASE_CONDUCTOR_CONFIG_CI: &str =
                include_str!("../../../conductor-config/conductor-config-ci.yaml");

            static CHC_CONDUCTOR_CONFIG: &str =
                include_str!("../../../conductor-config/with_chc.yaml");

            match std::env::var("CONDUCTOR_CONFIG") {
                Ok(value) if value == "CI" => {
                    if std::env::var("CHC_ENABLED")
                        .map(|v| v == "1")
                        .unwrap_or(false)
                    {
                        return format!("{}\n{}", BASE_CONDUCTOR_CONFIG_CI, CHC_CONDUCTOR_CONFIG);
                    }
                    BASE_CONDUCTOR_CONFIG_CI.into()
                }
                _ => {
                    if std::env::var("CHC_ENABLED")
                        .map(|v| v == "1")
                        .unwrap_or(false)
                    {
                        return format!("{}\n{}", BASE_CONDUCTOR_CONFIG, CHC_CONDUCTOR_CONFIG);
                    }
                    BASE_CONDUCTOR_CONFIG.into()
                }
            }
        }
    };
}
