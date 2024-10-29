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

            let mut config = if std::env::var("CONDUCTOR_CONFIG")
                .map(|value| value == "CI")
                .unwrap_or(false)
            {
                BASE_CONDUCTOR_CONFIG_CI.to_owned()
            } else {
                BASE_CONDUCTOR_CONFIG.to_owned()
            };

            if std::env::var("CHC_ENABLED")
                .map(|v| v == "1")
                .unwrap_or(false)
            {
                config = format!("{}\n{}", config, CHC_CONDUCTOR_CONFIG);
            }

            config
        }
    };
}
