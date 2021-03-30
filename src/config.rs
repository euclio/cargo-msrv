use std::path::{Path, PathBuf};
use rust_releases::semver;

#[derive(Debug, Clone)]
pub struct CmdMatches<'a> {
    target: String,
    check_command: Vec<&'a str>,
    seek_path: Option<PathBuf>,
    include_all_patch_releases: bool,
    minimum_version: Option<semver::Version>,
    maximum_version: Option<semver::Version>,
}

impl<'a> CmdMatches<'a> {
    pub fn new(target: String) -> Self {
        Self {
            target,
            check_command: vec!["cargo", "build", "--all"],
            seek_path: None,
            include_all_patch_releases: false,
            minimum_version: None,
            maximum_version: None,
        }
    }

    pub fn target(&self) -> &String {
        &self.target
    }

    pub fn check_command(&self) -> &Vec<&'a str> {
        &self.check_command
    }

    pub fn seek_path(&self) -> Option<&Path> {
        self.seek_path.as_deref()
    }

    pub fn include_all_patch_releases(&self) -> bool {
        self.include_all_patch_releases
    }

    pub fn minimum_version(&self) -> Option<&semver::Version> {
        self.minimum_version.as_ref()
    }

    pub fn maximum_version(&self) -> Option<&semver::Version> {
        self.maximum_version.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct CmdMatchesBuilder<'a> {
    inner: CmdMatches<'a>,
}

impl<'a> CmdMatchesBuilder<'a> {
    pub fn new(default_target: &str) -> Self {
        Self {
            inner: CmdMatches::new(default_target.to_string()),
        }
    }

    pub fn target(mut self, target: &str) -> Self {
        self.inner.target = target.to_string();
        self
    }

    pub fn check_command(mut self, cmd: Vec<&'a str>) -> Self {
        self.inner.check_command = cmd;
        self
    }

    pub fn seek_path<P: AsRef<Path>>(mut self, path: Option<P>) -> Self {
        self.inner.seek_path = path.map(|p| PathBuf::from(p.as_ref()));
        self
    }

    pub fn include_all_patch_releases(mut self, answer: bool) -> Self {
        self.inner.include_all_patch_releases = answer;
        self
    }

    pub fn minimum_version(mut self, version: Option<semver::Version>) -> Self {
        self.inner.minimum_version = version;
        self
    }

    pub fn maximum_version(mut self, version: Option<semver::Version>) -> Self {
        self.inner.maximum_version = version;
        self
    }

    pub fn build(self) -> CmdMatches<'a> {
        self.inner
    }
}
