use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt::{Display, Formatter};

use decent_toml_rs_alternative::TomlValue;

pub type TomlMap = HashMap<String, TomlValue>;

pub trait TomlParser {
    type Error;

    fn try_parse<T: TryFrom<TomlMap, Error = Self::Error>>(
        &self,
        contents: &str,
    ) -> Result<T, Self::Error>;

    fn parse<T: From<TomlMap>>(&self, contents: &str) -> Result<T, Self::Error>;
}

/// A structure for owning the values in a `Cargo.toml` manifest relevant for `cargo-msrv`.
#[derive(Debug)]
pub struct CargoManifest {
    minimum_rust_version: Option<BareVersion>,
}

impl CargoManifest {
    pub fn minimum_rust_version(&self) -> Option<&BareVersion> {
        self.minimum_rust_version.as_ref()
    }
}

/// A parser for `Cargo.toml` files. Only handles the parts necessary for `cargo-msrv`.
#[derive(Debug)]
pub struct CargoManifestParser;

impl Default for CargoManifestParser {
    fn default() -> Self {
        Self
    }
}

impl TomlParser for CargoManifestParser {
    type Error = crate::CargoMSRVError;

    fn try_parse<T: TryFrom<TomlMap, Error = Self::Error>>(
        &self,
        contents: &str,
    ) -> Result<T, Self::Error> {
        decent_toml_rs_alternative::parse_toml(contents)
            .map_err(crate::CargoMSRVError::ParseToml)
            .and_then(TryFrom::try_from)
    }

    fn parse<T: From<TomlMap>>(&self, contents: &str) -> Result<T, Self::Error> {
        decent_toml_rs_alternative::parse_toml(contents)
            .map_err(crate::CargoMSRVError::ParseToml)
            .map(From::from)
    }
}

impl TryFrom<TomlMap> for CargoManifest {
    type Error = crate::CargoMSRVError;

    fn try_from(map: TomlMap) -> Result<Self, Self::Error> {
        let minimum_rust_version = minimum_rust_version(&map)?;

        Ok(Self {
            minimum_rust_version,
        })
    }
}

type BareVersionUsize = u64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BareVersion {
    TwoComponents(BareVersionUsize, BareVersionUsize),
    ThreeComponents(BareVersionUsize, BareVersionUsize, BareVersionUsize),
}

impl<'s> TryFrom<&'s str> for BareVersion {
    type Error = crate::CargoMSRVError;

    fn try_from(value: &'s str) -> Result<Self, Self::Error> {
        parse_bare_version(value)
    }
}

impl BareVersion {
    pub fn try_to_semver<'s, I>(
        &self,
        iter: I,
    ) -> Result<&'s crate::semver::Version, crate::CargoMSRVError>
    where
        I: IntoIterator<Item = &'s crate::semver::Version>,
    {
        let mut iter = iter.into_iter();

        let requirements = match self {
            Self::TwoComponents(major, minor) => crate::semver::Comparator {
                op: crate::semver::Op::Tilde,
                major: *major,
                minor: Some(*minor),
                patch: None,
                pre: crate::semver::Prerelease::EMPTY,
            },
            Self::ThreeComponents(major, minor, patch) => crate::semver::Comparator {
                op: crate::semver::Op::Tilde,
                major: *major,
                minor: Some(*minor),
                patch: Some(*patch),
                pre: crate::semver::Prerelease::EMPTY,
            },
        };

        iter.find(|version| requirements.matches(version))
            .ok_or_else(|| {
                let requirement = self.to_owned();
                let available = iter.map(|v| v.to_owned()).collect();
                crate::CargoMSRVError::NoVersionMatchesManifestMSRV(requirement, available)
            })
    }
}

impl Display for BareVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TwoComponents(major, minor) => f.write_fmt(format_args!("{}.{}", major, minor)),
            Self::ThreeComponents(major, minor, patch) => {
                f.write_fmt(format_args!("{}.{}.{}", major, minor, patch))
            }
        }
    }
}

fn minimum_rust_version(value: &TomlMap) -> Result<Option<BareVersion>, crate::CargoMSRVError> {
    match find_minimum_rust_version(value) {
        Some(ref version) => {
            let x = parse_bare_version(version.as_str())?;
            Ok(Some(x))
        }
        None => Ok(None),
    }
}

fn parse_bare_version(value: &str) -> Result<BareVersion, crate::CargoMSRVError> {
    let mut components = value.split('.');

    let major = components
        .next()
        .ok_or_else(|| crate::CargoMSRVError::UnableToParseBareVersion {
            version: value.to_string(),
            message: "Couldn't find first component".to_string(),
        })
        .and_then(|c| {
            c.parse()
                .map_err(crate::CargoMSRVError::UnableToParseBareVersionNumber)
        })?;

    let minor = components
        .next()
        .ok_or_else(|| crate::CargoMSRVError::UnableToParseBareVersion {
            version: value.to_string(),
            message: "Couldn't find second component".to_string(),
        })
        .and_then(|c| {
            c.parse()
                .map_err(crate::CargoMSRVError::UnableToParseBareVersionNumber)
        })?;

    let version = if let Some(patch) = components.next() {
        let until_pre_release_id = patch.find('-').unwrap_or(patch.len());
        let patch = &patch[..until_pre_release_id];

        let patch_num = patch
            .parse()
            .map_err(crate::CargoMSRVError::UnableToParseBareVersionNumber)?;
        BareVersion::ThreeComponents(major, minor, patch_num)
    } else {
        BareVersion::TwoComponents(major, minor)
    };

    if let Some(peek) = components.next() {
        return Err(crate::CargoMSRVError::UnableToParseBareVersion {
            version: value.to_string(),
            message: format!("Unexpected tokens at the end of version number: '{}'", peek),
        });
    }

    Ok(version)
}

/// Parse the minimum supported Rust version (MSRV) from `Cargo.toml` manifest data.
fn find_minimum_rust_version(map: &TomlMap) -> Option<String> {
    /// Parses the `MSRV` as supported by Cargo since Rust 1.56.0
    ///
    /// [`Cargo`]: https://doc.rust-lang.org/cargo/reference/manifest.html#the-rust-version-field
    fn find_rust_version(map: &TomlMap) -> Option<String> {
        map.get("package")
            .and_then(|field| field.get("rust-version"))
            .and_then(|value| value.as_string())
    }

    /// Parses the MSRV as supported by `cargo-msrv`, since prior to the release of Rust
    /// 1.56.0
    fn find_metadata_msrv(map: &TomlMap) -> Option<String> {
        map.get("package")
            .and_then(|field| field.get("metadata"))
            .and_then(|field| field.get("msrv"))
            .and_then(|value| value.as_string())
    }

    // Parse the MSRV from the `package.rust-version` key if it exists,
    // and try to fallback to our own `package.metadata.msrv` if it doesn't
    find_rust_version(map).or_else(|| find_metadata_msrv(map))
}

#[cfg(test)]
mod minimal_version_tests {
    use std::convert::TryFrom;

    use crate::manifest::{BareVersion, CargoManifest, CargoManifestParser, TomlMap, TomlParser};

    #[test]
    fn parse_toml() {
        let contents = r#"[package]
name = "some"
version = "0.1.0"
edition = "2018"

[dependencies]
"#;

        assert!(CargoManifestParser::default()
            .parse::<TomlMap>(contents)
            .is_ok());
    }

    #[test]
    fn parse_invalid_toml() {
        let contents = r#"-[package]
name = "some"
version = "0.1.0"
edition = "2018"

[dependencies]
"#;

        assert!(CargoManifestParser::default()
            .parse::<TomlMap>(contents)
            .is_err());
    }

    #[test]
    fn parse_no_minimum_rust_version() {
        let contents = r#"[package]
name = "some"
version = "0.1.0"
edition = "2018"

[dependencies]
"#;

        let manifest = CargoManifestParser::default()
            .parse::<TomlMap>(contents)
            .unwrap();

        let manifest = CargoManifest::try_from(manifest).unwrap();

        assert!(manifest.minimum_rust_version.is_none());
    }

    #[test]
    fn parse_rust_version_three_components() {
        let contents = r#"[package]
name = "some"
version = "0.1.0"
edition = "2018"
rust-version = "1.56.0"

[dependencies]
"#;

        let manifest = CargoManifestParser::default()
            .parse::<TomlMap>(contents)
            .unwrap();

        let manifest = CargoManifest::try_from(manifest).unwrap();
        let version = manifest.minimum_rust_version.unwrap();

        assert_eq!(version, BareVersion::ThreeComponents(1, 56, 0));
    }

    #[test]
    fn parse_rust_version_three_components_with_pre_release() {
        let contents = r#"[package]
name = "some"
version = "0.1.0"
edition = "2018"
rust-version = "1.56.0-nightly"

[dependencies]
"#;

        let manifest = CargoManifestParser::default()
            .parse::<TomlMap>(contents)
            .unwrap();

        let manifest = CargoManifest::try_from(manifest).unwrap();
        let version = manifest.minimum_rust_version.unwrap();

        assert_eq!(version, BareVersion::ThreeComponents(1, 56, 0));
    }

    #[test]
    fn parse_rust_version_two_components() {
        let contents = r#"[package]
name = "some"
version = "0.1.0"
edition = "2018"
rust-version = "1.56"

[dependencies]
"#;

        let manifest = CargoManifestParser::default()
            .parse::<TomlMap>(contents)
            .unwrap();

        let manifest = CargoManifest::try_from(manifest).unwrap();
        let version = manifest.minimum_rust_version.unwrap();

        assert_eq!(version, BareVersion::TwoComponents(1, 56));
    }

    #[yare::parameterized(
        empty = {""},
        one_component = {"1"},
        one_component_dot = {"1."},
        two_components_dot = {"1.1."},
        three_components_dot = {"1.1.1."},
        two_components_with_pre_release = {"1.1-nightly"},
        two_components_not_a_number = {"1.x"},
        three_components_not_a_number = {"1.1.x"},
        too_many_components = {"1.1.0.0"},
    )]
    fn parse_rust_version_faulty_versions(version: &str) {
        let contents = format!(
            r#"[package]
name = "some"
version = "0.1.0"
edition = "2018"
rust-version = "{}"

[dependencies]
"#,
            version
        );

        let manifest = CargoManifestParser::default()
            .parse::<TomlMap>(&contents)
            .unwrap();

        let manifest = CargoManifest::try_from(manifest);

        assert!(manifest.is_err())
    }

    #[test]
    fn parse_metadata_msrv_three_components() {
        let contents = r#"[package]
name = "some"
version = "0.1.0"
edition = "2018"

[package.metadata]
msrv = "1.51.0"

[dependencies]
"#;

        let manifest = CargoManifestParser::default()
            .parse::<TomlMap>(contents)
            .unwrap();

        let manifest = CargoManifest::try_from(manifest).unwrap();
        let version = manifest.minimum_rust_version.unwrap();

        assert_eq!(version, BareVersion::ThreeComponents(1, 51, 0));
    }

    #[test]
    fn parse_metadata_msrv_two_components() {
        let contents = r#"[package]
name = "some"
version = "0.1.0"
edition = "2018"

[package.metadata]
msrv = "1.51"

[dependencies]
"#;

        let manifest = CargoManifestParser::default()
            .parse::<TomlMap>(contents)
            .unwrap();

        let manifest = CargoManifest::try_from(manifest).unwrap();
        let version = manifest.minimum_rust_version.unwrap();

        assert_eq!(version, BareVersion::TwoComponents(1, 51));
    }

    #[yare::parameterized(
        empty = {""},
        one_component = {"1"},
        one_component_dot = {"1."},
        two_components_dot = {"1.1."},
        three_components_dot = {"1.1.1."},
        two_components_with_pre_release = {"1.1-nightly"},
        two_components_not_a_number = {"1.x"},
        three_components_not_a_number = {"1.1.x"},
        too_many_components = {"1.1.0.0"},
    )]
    fn parse_metadata_msrv_faulty_versions(version: &str) {
        let contents = format!(
            r#"[package]
name = "some"
version = "0.1.0"
edition = "2018"

[package.metadata]
msrv = "{}"

[dependencies]
"#,
            version
        );

        let manifest = CargoManifestParser::default()
            .parse::<TomlMap>(&contents)
            .unwrap();

        let manifest = CargoManifest::try_from(manifest);

        assert!(manifest.is_err())
    }
}

#[cfg(test)]
mod bare_version_tests {
    use std::iter::FromIterator;

    use rust_releases::{semver, Release, ReleaseIndex};
    use yare::parameterized;

    use crate::manifest::BareVersion;

    fn release_indices() -> ReleaseIndex {
        FromIterator::from_iter(vec![
            Release::new_stable(semver::Version::new(2, 56, 0)),
            Release::new_stable(semver::Version::new(1, 56, 0)),
            Release::new_stable(semver::Version::new(1, 55, 0)),
            Release::new_stable(semver::Version::new(1, 54, 2)),
            Release::new_stable(semver::Version::new(1, 54, 1)),
            Release::new_stable(semver::Version::new(1, 0, 0)),
        ])
    }

    #[parameterized(
        two_component_two_fifty_six = { "2.56", BareVersion::TwoComponents(2, 56) },
        three_component_two_fifty_six = { "2.56.0", BareVersion::ThreeComponents(2, 56, 0) },
        two_component_one_fifty_five = { "1.55", BareVersion::TwoComponents(1, 55) },
        three_component_one_fifty_five = { "1.55.0", BareVersion::ThreeComponents(1, 55, 0) },
        three_component_one_fifty_four = { "1.54.0", BareVersion::ThreeComponents(1, 54, 0) },
        three_component_one_fifty_four_p1 = { "1.54.1", BareVersion::ThreeComponents(1, 54, 1) },
        three_component_one_fifty_four_p10 = { "1.54.10", BareVersion::ThreeComponents(1, 54, 10) },
        two_component_zeros = { "0.0", BareVersion::TwoComponents(0, 0) },
        three_component_zeros = { "0.0.0", BareVersion::ThreeComponents(0, 0, 0) },
        two_component_large_major = { "18446744073709551615.0", BareVersion::TwoComponents(18446744073709551615, 0) },
        two_component_large_minor = { "0.18446744073709551615", BareVersion::TwoComponents(0, 18446744073709551615) },
        three_component_large_major = { "18446744073709551615.0.0", BareVersion::ThreeComponents(18446744073709551615, 0, 0) },
        three_component_large_minor = { "0.18446744073709551615.0", BareVersion::ThreeComponents(0, 18446744073709551615, 0) },
        three_component_large_patch = { "0.0.18446744073709551615", BareVersion::ThreeComponents(0, 0, 18446744073709551615) },
        // two_component_pre_release_id_variant_1 = { "0.0-nightly", BareVersion::TwoComponents(0, 0) }, // FIXME: allow pre release identifiers in two component versions
        // two_component_pre_release_id_variant_2 = { "0.0-beta.0", BareVersion::TwoComponents(0, 0) }, // FIXME: parse versions properly with Lr tokens
        // two_component_pre_release_id_variant_3 = { "0.0-beta.1", BareVersion::TwoComponents(0, 0) }, // FIXME: parse versions properly with Lr tokens
        // two_component_pre_release_id_variant_4 = { "0.0-anything", BareVersion::TwoComponents(0, 0) }, // FIXME: allow pre release identifiers in two component versions
        // two_component_pre_release_id_variant_5 = { "0.0-anything+build", BareVersion::TwoComponents(0, 0) }, // FIXME: allow pre release identifiers in two component versions
        three_component_pre_release_id_variant_1 = { "0.0.0-nightly", BareVersion::ThreeComponents(0, 0, 0) },
        // three_component_pre_release_id_variant_2 = { "0.0.0-beta.0", BareVersion::ThreeComponents(0, 0, 0) }, // FIXME: parse versions properly with Lr tokens
        // three_component_pre_release_id_variant_3 = { "0.0.0-beta.1", BareVersion::ThreeComponents(0, 0, 0) }, // FIXME: parse versions properly with Lr tokens
        three_component_pre_release_id_variant_4 = { "0.0.0-anything", BareVersion::ThreeComponents(0, 0, 0) }, 
        three_component_pre_release_id_variant_5 = { "0.0.0-anything+build", BareVersion::ThreeComponents(0, 0, 0) },
    )]
    fn try_from_ok(version: &str, expected: BareVersion) {
        use std::convert::TryFrom;

        let version = BareVersion::try_from(version).unwrap();

        assert_eq!(version, expected);
    }

    #[parameterized(
        empty = { "" }, // no first component
        no_components_space = { "1 36 0" },
        no_components_comma = { "1,36,0" },
        first_component_nan = { "x.0.0" },
        no_second_component = { "1." },
        second_component_nan = { "1.x" },
        no_third_component = { "1.0." },
        third_component_nan = { "1.36.x" },
        too_large_int_major_2c = { "18446744073709551616.0" },
        too_large_int_minor_2c = { "0.18446744073709551616" },
        too_large_int_major_3c = { "18446744073709551616.0.0" },
        too_large_int_minor_3c = { "0.18446744073709551616.0" },
        too_large_int_patch_3c = { "0.0.18446744073709551616" },        
        neg_int_major = { "-1.0.0" },
        neg_int_minor = { "0.-1.0" },
        neg_int_patch = { "0.0.-1" },
        build_postfix_without_pre_release_id = { "0.0.0+some" },
    )]
    fn try_from_err(version: &str) {
        use std::convert::TryFrom;

        let res = BareVersion::try_from(version);

        assert!(res.is_err());
    }

    #[parameterized(
        two_fifty_six = {  BareVersion::TwoComponents(2, 56), semver::Version::new(2, 56, 0) },
        one_fifty_six = {  BareVersion::TwoComponents(1, 56), semver::Version::new(1, 56, 0) },
        one_fifty_five = {  BareVersion::TwoComponents(1, 55), semver::Version::new(1, 55, 0) },
        one_fifty_four_p2 = {  BareVersion::TwoComponents(1, 54), semver::Version::new(1, 54, 2) },
        one_fifty_four_p1 = {  BareVersion::TwoComponents(1, 54), semver::Version::new(1, 54, 2) },
        one_fifty_four_p0 = {  BareVersion::TwoComponents(1, 54), semver::Version::new(1, 54, 2) },
        one = {  BareVersion::TwoComponents(1, 0), semver::Version::new(1, 0, 0) },
    )]
    fn two_components_to_semver(version: BareVersion, expected: semver::Version) {
        let index = release_indices();
        let available = index.releases().iter().map(|release| release.version());

        let v = version.try_to_semver(available).unwrap();

        assert_eq!(v, &expected);
    }

    #[parameterized(
        two_fifty_six = {  BareVersion::ThreeComponents(2, 56, 0), semver::Version::new(2, 56, 0) },
        one_fifty_six = {  BareVersion::ThreeComponents(1, 56, 0), semver::Version::new(1, 56, 0) },
        one_fifty_five = {  BareVersion::ThreeComponents(1, 55, 0), semver::Version::new(1, 55, 0) },
        one_fifty_four_p2 = {  BareVersion::ThreeComponents(1, 54, 2), semver::Version::new(1, 54, 2) },
        one_fifty_four_p1 = {  BareVersion::ThreeComponents(1, 54, 1), semver::Version::new(1, 54, 2) },
        one_fifty_four_p0 = {  BareVersion::ThreeComponents(1, 54, 0), semver::Version::new(1, 54, 2) },
        one = {  BareVersion::ThreeComponents(1, 0, 0), semver::Version::new(1, 0, 0) },
    )]
    fn three_components_to_semver(version: BareVersion, expected: semver::Version) {
        let index = release_indices();
        let available = index.releases().iter().map(|release| release.version());

        let v = version.try_to_semver(available).unwrap();

        assert_eq!(v, &expected);
    }
}
