use crate::config::Config;
use crate::errors::{CargoMSRVError, TResult};
use cargo_metadata::MetadataCommand;
use crate::paths::crate_root_folder;

trait DependencyResolver {
    fn resolve(&self) -> TResult<Dependencies>;
}

struct Dependencies {
    packages: Vec<cargo_metadata::Package>,
}

struct Dependency {
    name: String,
    dependency: cargo_metadata::Dependency,
}

struct CargoMetadataResolver {
    metadata_command: MetadataCommand,
}

impl CargoMetadataResolver {
    pub fn try_from_config(config: &Config) -> TResult<Self> {
        let crate_root = crate_root_folder(config)?;

        let mut metadata_command = MetadataCommand::new();
        metadata_command.manifest_path(crate_root);

        Ok(Self {
            metadata_command,
        })
    }
}

impl DependencyResolver for CargoMetadataResolver {
    fn resolve(&self) -> TResult<Dependencies> {
        let result = self.metadata_command.exec()
            .map_err(CargoMSRVError::CargoMetadata)?;

        result.packages.into_iter()
            .map(|pkg| )

        Ok()
    }
}

