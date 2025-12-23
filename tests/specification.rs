fn main() -> std::process::ExitCode {
    let output = std::process::Command::new("spectra")
        .arg("test")
        .arg("./specification.md")
        .arg("./target/debug/examples/parse --rpc --interactive")
        .status()
        .unwrap();

    if output.code().is_none_or(|item| item == 0) {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::FAILURE
    }
}
