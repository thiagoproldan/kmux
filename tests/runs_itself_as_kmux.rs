//! Fork gate: kmux runs itself as `kmux`, and lives in its own socket directory.
//!
//! kmux is ztmux under another name (README.md, Provenance). Every command line
//! the code hands a shell to run this binary -- the default key bindings and
//! hooks, the modal tables, the popup an extension verb expands to, the trigger
//! wizard -- must name `kmux`: a machine with only kmux installed has no `ztmux`
//! to run. A merge from upstream brings new such lines naming `ztmux`, and this
//! gate fails at that merge.
//!
//! The rule is checked over the sources, every code line with its `//` comment
//! stripped, and from inside a running server, whose bindings and hooks are what
//! a user's keys reach. The server also shows where its socket lives: in
//! `kmux-<uid>`, not in ztmux's `ztmux-<uid>`, so a kmux and a ztmux on one
//! machine never share a server.

use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_kmux");

/// How a command line runs the binary against a server: by name, with the
/// server's socket.
const RUNS_UPSTREAM: &[&str] = &["ztmux -S", "ztmux -L"];
const RUNS_ITSELF: &str = "kmux -S";

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            rs_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// Strip a `//` comment, so a name merely mentioned in prose never counts.
fn code(line: &str) -> &str {
    match line.find("//") {
        Some(i) => &line[..i],
        None => line,
    }
}

#[test]
fn no_code_line_runs_ztmux() {
    let mut files = Vec::new();
    rs_files(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut files);
    let mut upstream = Vec::new();
    let mut ours = 0;
    for f in &files {
        let text = fs::read_to_string(f).expect("read a source file");
        for (n, line) in text.lines().enumerate() {
            let code = code(line);
            if RUNS_UPSTREAM.iter().any(|p| code.contains(p)) {
                upstream.push(format!("{}:{}: {}", f.display(), n + 1, line.trim()));
            }
            ours += code.matches(RUNS_ITSELF).count();
        }
    }
    assert!(
        upstream.is_empty(),
        "these lines run `ztmux`, which kmux's machines lack; name `kmux`:\n{}",
        upstream.join("\n")
    );
    // The bindings that run the binary are there to be read: none found means
    // this gate reads the wrong text, not that the rule holds.
    assert!(ours > 10, "only {ours} code lines run `{RUNS_ITSELF}` in {} files", files.len());
}

#[test]
fn a_server_lives_in_kmux_uid_and_its_keys_and_hooks_run_kmux() {
    let tmp = std::env::temp_dir().join(format!("kmux-gate-{}", std::process::id()));
    fs::create_dir_all(&tmp).expect("make a TMUX_TMPDIR");
    let uid = fs::metadata(&tmp).expect("stat it").uid();
    let kmux = |args: &[&str]| -> Output {
        Command::new(BIN)
            .env("TMUX_TMPDIR", &tmp)
            .env_remove("TMUX")
            .args(["-L", "gate", "-f", "/dev/null"])
            .args(args)
            .output()
            .expect("run kmux")
    };
    let started = kmux(&["new-session", "-d", "-s", "gate"]).status.success();
    let socket = tmp.join(format!("kmux-{uid}")).join("gate");
    let there = socket.exists();
    let keys = String::from_utf8_lossy(&kmux(&["list-keys"]).stdout).into_owned();
    let hooks = String::from_utf8_lossy(&kmux(&["show-hooks", "-g"]).stdout).into_owned();
    let _ = kmux(&["kill-server"]);
    let _ = fs::remove_dir_all(&tmp);

    assert!(started, "no server started");
    assert!(there, "no socket at {}", socket.display());
    for (what, text) in [("list-keys", &keys), ("show-hooks -g", &hooks)] {
        let upstream: Vec<&str> = text
            .lines()
            .filter(|l| RUNS_UPSTREAM.iter().any(|p| l.contains(p)))
            .collect();
        assert!(upstream.is_empty(), "{what} runs `ztmux`:\n{}", upstream.join("\n"));
        assert!(text.contains(RUNS_ITSELF), "{what} runs no `{RUNS_ITSELF}`:\n{text}");
    }
}

#[test]
fn version_names_kmux() {
    let out = Command::new(BIN).arg("-V").output().expect("run kmux -V");
    let said = String::from_utf8_lossy(&out.stdout);
    assert!(said.starts_with("kmux "), "kmux -V said {said:?}");
}
