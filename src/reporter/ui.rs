use std::borrow::Cow;

use console::{style, Term};
use indicatif::{ProgressBar, ProgressStyle};
use rust_releases::semver;

use crate::config::ModeIntent;

pub struct HumanPrinter<'s, 't> {
    term: Term,
    progress: ProgressBar,
    toolchain: &'s str,
    cmd: &'t str,
}

impl std::fmt::Debug for HumanPrinter<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "toolchain = {}, cmd = {}",
            self.toolchain, self.cmd
        ))
    }
}

impl<'s, 't> HumanPrinter<'s, 't> {
    pub fn new(steps: u64, toolchain: &'s str, cmd: &'t str) -> Self {
        let term = Term::stderr();

        let progress = ProgressBar::new(steps).with_style(
            ProgressStyle::default_spinner()
                .template(" {spinner} {msg:<30} {wide_bar} {elapsed_precise}"),
        );

        Self {
            term,
            progress,
            toolchain,
            cmd,
        }
    }

    fn welcome(&self, target: &str, cmd: &str, action_intent: ModeIntent) {
        let verb = match action_intent {
            ModeIntent::DetermineMSRV => "Determining",
            ModeIntent::VerifyMSRV => "Verifying",
            ModeIntent::List => "",
        };

        let _ = self.term.write_line(
            format!(
                "{} the Minimum Supported Rust Version (MSRV) for toolchain {}",
                verb,
                style(target).bold()
            )
            .as_str(),
        );

        let _ = self.term.write_line(
            format!(
                "Using {} command {}",
                style("check").bold(),
                style(cmd).italic(),
            )
            .as_str(),
        );

        self.progress.enable_steady_tick(250);
    }

    fn show_progress(&self, action: &str, version: &semver::Version) {
        self.progress.set_message(format!(
            "{} {}",
            style(action).green().bold(),
            style(version).cyan()
        ));
    }

    fn set_progress_bar_length(&self, len: u64) {
        self.progress.set_length(len)
    }

    fn complete_step(&self, message: impl Into<Cow<'static, str>>) {
        self.progress.set_message(message);
        self.progress.inc(1);
    }

    // for DetermineMSRV
    fn finish_with_ok(&self, message: &str, version: &semver::Version) {
        self.progress.finish_with_message(format!(
            "{} {} {}",
            style("Finished").green().bold(),
            message,
            style(version).cyan()
        ))
    }

    fn finish_with_err(&self, cmd: &str) {
        self.progress.abandon();
        let _ = self.term.write_line(
            format!(
                "   {} {} command {} didn't succeed",
                style("Failed").red().bold(),
                style("check").bold(),
                style(cmd).italic()
            )
            .as_str(),
        );
    }
}

impl<'s, 't> crate::Output for HumanPrinter<'s, 't> {
    fn mode(&self, action: ModeIntent) {
        self.welcome(self.toolchain, self.cmd, action);
    }

    fn set_steps(&self, steps: u64) {
        self.set_progress_bar_length(steps);
    }

    fn progress(&self, action: crate::ProgressAction) {
        let (action, version) = match action {
            crate::ProgressAction::Installing(version) => ("Installing", Some(version)),
            crate::ProgressAction::Checking(version) => ("Checking", Some(version)),
            crate::ProgressAction::FetchingIndex => ("Fetching index", None),
        };

        if let Some(version) = version {
            self.show_progress(action, version);
        } else {
            let _ = self.term.write_line(action);
        }
    }

    fn complete_step(&self, version: &semver::Version, success: bool) {
        if success {
            self.complete_step(format!(
                "{} Good check for {}",
                style("Done").green().bold(),
                style(version).cyan()
            ));
        } else {
            self.complete_step(format!(
                "{} Bad check for {}",
                style("Done").green().bold(),
                style(version).cyan()
            ));
        }
    }

    fn finish_success(&self, mode: ModeIntent, version: &semver::Version) {
        match mode {
            ModeIntent::DetermineMSRV => self.finish_with_ok("The MSRV is:", version),
            ModeIntent::VerifyMSRV => self.finish_with_ok("Satisfied MSRV check:", version),
            ModeIntent::List => {}
        }
    }

    fn finish_failure(&self, _mode: ModeIntent, cmd: &str) {
        self.finish_with_err(cmd)
    }
}
