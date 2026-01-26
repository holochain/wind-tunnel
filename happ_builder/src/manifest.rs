mod sha256;

use serde::Deserialize;

pub use self::sha256::Sha256Hash;

/// A toml helper to allow for [`Metadata`] fields to be either a single item or a list of items
///
/// This mean it will both allow:
///
/// ```toml
/// [package.metadata.required-dna]
/// name = "timed_and_validated"
/// zomes = ["timed_and_validated"]
/// ```
///
/// and
///
/// ```toml
/// [[package.metadata.required-dna]]
/// name = "timed_and_validated"
/// zomes = ["timed_and_validated"]
///
/// [[package.metadata.required-dna]]
/// name = "foo"
/// zomes = ["foo"]
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum OneOrMany<T> {
    One(T),
    Many(Vec<T>),
}

impl<T> Default for OneOrMany<T> {
    fn default() -> Self {
        OneOrMany::Many(Vec::new())
    }
}

/// Cargo manifest keys to fetch and build hApps from
#[derive(Debug, Default, Clone, Deserialize)]
pub struct CargoToml {
    #[serde(default, rename = "package")]
    pub package: Package,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct Package {
    #[serde(default, rename = "metadata")]
    pub metadata: Metadata,
}

/// Cargo manifest metadata for hApp fetching and building
#[derive(Debug, Default, Clone, Deserialize)]
pub struct Metadata {
    #[serde(rename = "fetch-required-happ", default)]
    fetch_required_happ: OneOrMany<FetchRequiredHapp>,
    #[serde(rename = "required-dna", default)]
    required_dnas: OneOrMany<RequiredDna>,
    #[serde(rename = "required-happ", default)]
    required_happs: OneOrMany<RequiredHapp>,
}

impl Metadata {
    pub fn fetch_required_happ(&self) -> Vec<FetchRequiredHapp> {
        match &self.fetch_required_happ {
            OneOrMany::One(item) => vec![item.clone()],
            OneOrMany::Many(items) => items.clone(),
        }
    }

    pub fn required_dnas(&self) -> Vec<RequiredDna> {
        match &self.required_dnas {
            OneOrMany::One(item) => vec![item.clone()],
            OneOrMany::Many(items) => items.clone(),
        }
    }

    pub fn required_happs(&self) -> Vec<RequiredHapp> {
        match &self.required_happs {
            OneOrMany::One(item) => vec![item.clone()],
            OneOrMany::Many(items) => items.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct FetchRequiredHapp {
    pub name: String,
    pub url: String,
    pub sha256: Sha256Hash,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RequiredDna {
    pub name: String,
    pub zomes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RequiredHapp {
    pub name: String,
    pub dnas: Vec<String>,
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn should_parse_manifest() {
        let manifest: CargoToml =
            toml::from_str(TEST_MANIFEST).expect("Failed to parse test manifest");

        let metadata = &manifest.package.metadata;

        let fetch_required_happs = metadata.fetch_required_happ();
        assert_eq!(fetch_required_happs.len(), 2);
        assert_eq!(fetch_required_happs[0].name, "foo");
        assert_eq!(
            fetch_required_happs[0].url,
            "https://github.com/holochain/happs/foo.happ"
        );

        assert_eq!(
            fetch_required_happs[0].sha256,
            Sha256Hash::try_from(
                hex::decode("1eafa0d852d9e96e54f0b6969fb06de83989ece0059bc2b376884ac52fb6a63a")
                    .expect("Invalid hex string")
                    .as_slice()
            )
            .expect("Invalid SHA256"),
        );
        assert_eq!(fetch_required_happs[1].name, "bar");
        assert_eq!(
            fetch_required_happs[1].url,
            "https://github.com/holochain/happs/bar.happ"
        );

        let required_dnas = metadata.required_dnas();
        assert_eq!(required_dnas.len(), 2);
        assert_eq!(required_dnas[0].name, "timed_and_validated");
        assert_eq!(required_dnas[0].zomes, vec!["timed_and_validated"]);
        assert_eq!(required_dnas[1].name, "foo");
        assert_eq!(required_dnas[1].zomes, vec!["foo_zome", "common_zome"]);

        let required_happs = metadata.required_happs();
        assert_eq!(required_happs.len(), 1);
        assert_eq!(required_happs[0].name, "timed_and_validated");
        assert_eq!(required_happs[0].dnas, vec!["timed_and_validated"]);
    }

    #[test]
    fn should_parse_empty_metadata() {
        let manifest: CargoToml = toml::from_str(EMPTY_MANIFEST_METADATA)
            .expect("Failed to parse empty manifest metadata");

        let metadata = &manifest.package.metadata;
        let fetch_required_happs = metadata.fetch_required_happ();
        assert_eq!(fetch_required_happs.len(), 0);
        let required_dnas = metadata.required_dnas();
        assert_eq!(required_dnas.len(), 0);
        let required_happs = metadata.required_happs();
        assert_eq!(required_happs.len(), 0);
    }

    #[test]
    fn should_parse_empty_manifest() {
        let manifest: CargoToml =
            toml::from_str(EMPTY_MANIFEST).expect("Failed to parse empty manifest");
        let metadata = &manifest.package.metadata;
        let fetch_required_happs = metadata.fetch_required_happ();
        assert_eq!(fetch_required_happs.len(), 0);
        let required_dnas = metadata.required_dnas();
        assert_eq!(required_dnas.len(), 0);
        let required_happs = metadata.required_happs();
        assert_eq!(required_happs.len(), 0);
    }

    #[test]
    fn should_parse_missing_keys() {
        let manifest: CargoToml = toml::from_str(TEST_MANIFEST_WITH_MISSING_KEYS)
            .expect("Failed to parse empty manifest");
        let metadata = &manifest.package.metadata;
        let fetch_required_happs = metadata.fetch_required_happ();
        assert_eq!(fetch_required_happs.len(), 2);
        let required_dnas = metadata.required_dnas();
        assert_eq!(required_dnas.len(), 0);
        let required_happs = metadata.required_happs();
        assert_eq!(required_happs.len(), 0);
    }

    const TEST_MANIFEST: &str = r#"
# to be fetched (Many)
[[package.metadata.fetch-required-happ]]
name = "foo"
url = "https://github.com/holochain/happs/foo.happ"
sha256 = "1eafa0d852d9e96e54f0b6969fb06de83989ece0059bc2b376884ac52fb6a63a"

[[package.metadata.fetch-required-happ]]
name = "bar"
url = "https://github.com/holochain/happs/bar.happ"
sha256 = "2eafa0d852d9e96e54f0b6969fb06de83989ece0059bc2b376884ac52fb6a63b"

# to be built (Many)
[[package.metadata.required-dna]]
name = "timed_and_validated"
zomes = ["timed_and_validated"]

[[package.metadata.required-dna]]
name = "foo"
zomes = ["foo_zome", "common_zome"]

# to be built (One)
[package.metadata.required-happ]
name = "timed_and_validated"
dnas = ["timed_and_validated"]
"#;

    const TEST_MANIFEST_WITH_MISSING_KEYS: &str = r#"
# to be fetched (Many)
[[package.metadata.fetch-required-happ]]
name = "foo"
url = "https://github.com/holochain/happs/foo.happ"
sha256 = "1eafa0d852d9e96e54f0b6969fb06de83989ece0059bc2b376884ac52fb6a63a"

[[package.metadata.fetch-required-happ]]
name = "bar"
url = "https://github.com/holochain/happs/bar.happ"
sha256 = "2eafa0d852d9e96e54f0b6969fb06de83989ece0059bc2b376884ac52fb6a63b"
"#;

    const EMPTY_MANIFEST_METADATA: &str = r#"
# empty metadata
[package.metadata]
"#;

    const EMPTY_MANIFEST: &str = r#"
# empty manifest
"#;
}
