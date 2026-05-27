use std::env;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

const USAGE: &str = "\
Usage:
  cargo xtask test [all|unit|e2e|dotnet|python]...
  cargo xtask generate-bindings
  cargo xtask help
";

#[derive(Debug, Clone, Copy)]
struct TestSelection {
    unit: bool,
    dotnet: bool,
    python: bool,
}

impl TestSelection {
    fn from_targets(targets: &[String]) -> Result<Self, String> {
        let mut selection = Self {
            unit: false,
            dotnet: false,
            python: false,
        };

        if targets.is_empty() {
            selection.unit = true;
            selection.dotnet = true;
            selection.python = true;
            return Ok(selection);
        }

        for target in targets {
            match target.as_str() {
                "all" => {
                    selection.unit = true;
                    selection.dotnet = true;
                    selection.python = true;
                }
                "unit" => selection.unit = true,
                "e2e" => {
                    selection.dotnet = true;
                    selection.python = true;
                }
                "dotnet" => selection.dotnet = true,
                "python" => selection.python = true,
                unknown => {
                    return Err(format!(
                        "unknown test target `{unknown}`. Expected all, unit, e2e, dotnet, or python."
                    ));
                }
            }
        }

        Ok(selection)
    }

    fn needs_e2e_setup(self) -> bool {
        self.dotnet || self.python
    }
}

fn main() -> ExitCode {
    match try_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn try_main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        print!("{USAGE}");
        return Ok(());
    };

    let rest = args.collect::<Vec<_>>();
    match command.as_str() {
        "test" => run_tests(&rest),
        "generate-bindings" => generate_bindings(),
        "help" | "-h" | "--help" => {
            print!("{USAGE}");
            Ok(())
        }
        unknown => Err(format!("unknown command `{unknown}`.\n\n{USAGE}")),
    }
}

fn run_tests(targets: &[String]) -> Result<(), String> {
    let repo_root = repo_root();
    let selection = TestSelection::from_targets(targets)?;

    heading("Building core oxidizer crates");
    run(&repo_root, "cargo", ["build"])?;

    if selection.needs_e2e_setup() {
        build_rust_lib(&repo_root)?;
        generate_bindings()?;
    }

    if selection.unit {
        heading("Running Cargo unit tests");
        run(&repo_root, "cargo", ["test", "--workspace"])?;
    }

    if selection.dotnet {
        heading("Running C# (xUnit) tests");
        let project = repo_root
            .join("tests")
            .join("e2e")
            .join("dotnet")
            .join("DotnetTests.csproj");
        run(
            &repo_root,
            "dotnet",
            [
                "test".into(),
                project.into_os_string(),
                "--verbosity".into(),
                "normal".into(),
            ],
        )?;
    }

    if selection.python {
        heading("Running Python (pytest) tests");
        let python = find_python()?;
        let test_dir = repo_root.join("tests").join("e2e").join("python");
        run(
            &repo_root,
            &python,
            [
                "-m".into(),
                "pytest".into(),
                test_dir.into_os_string(),
                "-v".into(),
            ],
        )?;
    }

    heading("All selected tests passed");
    Ok(())
}

fn build_rust_lib(repo_root: &Path) -> Result<(), String> {
    heading("Building rust_lib DLL");
    run(repo_root, "cargo", ["build", "-p", "rust_lib"])
}

fn generate_bindings() -> Result<(), String> {
    heading("Generating bindings");
    run(&repo_root(), "cargo", ["run", "-p", "bindings-generator"])
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask should live directly under the workspace root")
        .to_path_buf()
}

fn heading(message: &str) {
    println!("\n=== {message} ===");
}

fn find_python() -> Result<OsString, String> {
    for candidate in ["python", "python3", "py"] {
        if command_succeeds(candidate, ["--version"]) {
            return Ok(candidate.into());
        }
    }

    Err("Python was not found. Install Python or ensure python/python3/py is on PATH.".into())
}

fn command_succeeds<I, S>(program: &str, args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn run<I, P, S>(cwd: &Path, program: P, args: I) -> Result<(), String>
where
    I: IntoIterator<Item = S>,
    P: AsRef<OsStr>,
    S: AsRef<OsStr>,
{
    let program_ref = program.as_ref();
    let mut command = Command::new(program_ref);
    command.current_dir(cwd).args(args);

    let status = command.status().map_err(|err| {
        format!(
            "failed to start `{}` in {}: {err}",
            program_ref.to_string_lossy(),
            cwd.display()
        )
    })?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "`{}` failed with status {status}",
            program_ref.to_string_lossy()
        ))
    }
}
