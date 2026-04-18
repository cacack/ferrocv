use std::process::ExitCode;

fn main() -> ExitCode {
    ferrocv::cli::run().unwrap_or_else(|err| {
        eprintln!("error: {err:#}");
        ExitCode::from(2)
    })
}
