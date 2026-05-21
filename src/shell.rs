use anyhow::Context;
use std::ffi::OsString;
use std::io::Write;

pub(crate) fn render_top_level_help() -> anyhow::Result<()> {
    write_stdout(format_args!("Velvet Ballistics xtask commands"))?;
    write_stdout(format_args!(""))?;
    write_stdout(format_args!("Usage: xtask <COMMAND> [OPTIONS]"))?;
    write_stdout(format_args!(""))?;
    write_stdout(format_args!("Required command families:"))?;
    for spec in xtask::required_command_families() {
        write_stdout(format_args!("  {}", spec.public_name()))?;
    }
    write_stdout(format_args!(""))?;
    write_stdout(format_args!("Legacy commands:"))?;
    write_stdout(format_args!("  ui-snapshot"))?;
    write_stdout(format_args!("  ui-tokens"))?;
    write_stdout(format_args!("  ui-overlap-check"))?;
    write_stdout(format_args!("  ai-fast"))?;
    write_stdout(format_args!("  ai-deep"))?;
    write_stdout(format_args!("  ai-release"))?;
    write_stdout(format_args!(""))?;
    write_stdout(format_args!("Top-level options:"))?;
    write_stdout(format_args!("  -h, --help     Print help"))?;
    write_stdout(format_args!("  -V, --version  Print version"))
}

pub(crate) fn render_top_level_version() -> anyhow::Result<()> {
    write_stdout(format_args!("xtask {}", env!("CARGO_PKG_VERSION")))
}

pub(crate) fn run_required_command(command: xtask::CommandFamily) -> anyhow::Result<()> {
    let env = xtask::XtaskEnvironment {
        workspace_root: std::env::current_dir().context("Failed to read workspace root")?,
        bead_id: None,
        output_format: xtask::OutputFormat::JsonLines,
        unavailable_families: Vec::new(),
    };
    let status = xtask::route_command(xtask::XtaskCommand::Required(command), &env)
        .map_err(anyhow::Error::msg)?;
    let output =
        xtask::render_structured_status(&status, env.output_format).map_err(anyhow::Error::msg)?;
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    handle
        .write_all(output.as_bytes())
        .context("Failed to write structured status")?;
    Ok(())
}

pub(crate) fn exit_with_xtask_error(error: xtask::XtaskCommandError) -> anyhow::Result<()> {
    let stderr = std::io::stderr();
    let mut handle = stderr.lock();
    handle
        .write_fmt(format_args!("{error:?}; remediation: run xtask --help\n"))
        .context("Failed to write xtask error")?;
    std::process::exit(2);
}

pub(crate) fn normalized_args() -> Vec<OsString> {
    std::env::args_os()
        .enumerate()
        .filter_map(|(index, arg)| {
            let is_legacy_separator = index == 1 && arg == "--";
            (!is_legacy_separator).then_some(arg)
        })
        .collect()
}

pub(crate) fn write_stdout(args: std::fmt::Arguments<'_>) -> anyhow::Result<()> {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    handle
        .write_fmt(args)
        .context("Failed to write to stdout")?;
    handle
        .write_all(b"\n")
        .context("Failed to write newline to stdout")?;
    Ok(())
}
