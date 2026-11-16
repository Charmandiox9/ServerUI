use std::{
    io,
    process::{Command, Output},
};

fn command(args: &[&str]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_serverui"))
        .args(args)
        .output()
}

#[test]
fn help_and_version_work_without_a_terminal() -> io::Result<()> {
    let help = command(&["--help"])?;
    assert!(help.status.success());
    let text = String::from_utf8_lossy(&help.stdout);
    for flag in [
        "--demo",
        "--ascii",
        "--no-color",
        "--refresh-ms",
        "--demo-scenario",
    ] {
        assert!(text.contains(flag));
    }
    let version = command(&["--version"])?;
    assert!(version.status.success());
    assert!(String::from_utf8_lossy(&version.stdout).contains("0.1.0"));
    Ok(())
}

#[test]
fn unsupported_real_mode_fails_before_touching_terminal() -> io::Result<()> {
    let output = command(&[])?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("proveedor real"));
    assert!(!output.stderr.contains(&0x1b));
    assert!(output.stdout.is_empty());
    Ok(())
}

#[test]
fn invalid_arguments_are_rejected() -> io::Result<()> {
    for args in [
        vec!["--demo", "--refresh-ms", "0"],
        vec!["--demo", "--refresh-ms", "60001"],
        vec!["--unknown"],
        vec!["--demo-scenario", "error"],
        vec!["--demo", "--demo-scenario", "missing"],
    ] {
        let output = command(&args)?;
        assert!(!output.status.success(), "{args:?}");
        assert!(!output.stderr.contains(&0x1b));
    }
    Ok(())
}

#[test]
fn demo_requires_interactive_input_and_output() -> io::Result<()> {
    let output = command(&["--demo", "--ascii", "--no-color"])?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("TTY"));
    assert!(!output.stderr.contains(&0x1b));
    assert!(output.stdout.is_empty());
    Ok(())
}
