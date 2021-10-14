// TODO{foresterre}: support custom toolchains
#[derive(Debug)]
pub struct Toolchain {
    channel: ToolchainChannel,
    date: Date,
    host: TargetTriple,
    components: Vec<Component>,
}

#[derive(Debug)]
pub enum ToolchainChannel {
    Channel(rust_releases::Channel),
    Version(rust_releases::semver::Version),
}

#[derive(Debug)]
pub struct Date;

#[derive(Debug)]
pub struct TargetTriple;

#[derive(Debug)]
pub struct Component {
    id: String,
}
