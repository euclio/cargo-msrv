use cargo_msrv::config::Config;
use cargo_msrv::errors::{CargoMSRVError, TResult};
use cargo_msrv::reporter::ReporterBuilder;
use cargo_msrv::{cli, run_app};
use directories_next::ProjectDirs;
use std::convert::TryFrom;
use std::path::Path;
use tracing_appender::rolling::{RollingFileAppender, Rotation};

fn main() {
    if let Err(err) = init_and_run() {
        eprintln!("{}", err);
    }
}

#[tracing::instrument]
fn init_and_run() -> TResult<()> {
    let matches = cli::cli().get_matches();
    let config = Config::try_from(&matches)?;

    let log_folder = ProjectDirs::from("github", "foresterre", "cargo-msrv")
        .ok_or_else(|| CargoMSRVError::UnableToAccessLogFolder)?;

    if !config.no_tracing() {
        std::env::set_var("RUST_LOG", "INFO");
        init_tracing(log_folder.data_local_dir());
    }

    let target = config.target().as_str();
    let cmd = config.check_command_string();

    tracing::info!("Initializing reporter");
    let reporter = ReporterBuilder::new(target, cmd.as_str())
        .output_format(config.output_format())
        .build();

    tracing::info!("Running app");

    let _ = run_app(&config, &reporter)?;

    tracing::info!("Finished app");

    std::env::remove_var("RUST_LOG");

    Ok(())
}

fn init_tracing(log_folder: &Path) {
    let file_appender = RollingFileAppender::new(Rotation::NEVER, log_folder, "cargo-msrv.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt().with_writer(non_blocking).init();

    tracing::info!("Initialized tracing subscriber");
}
