//! The client's working directory is tmux's `find_cwd` (`vendor/tmux/tmux.c:323`):
//! `$PWD` when it names the directory the client runs in, so a path reached
//! through a symbolic link keeps the link; the real directory when `$PWD` is unset,
//! empty, or names another directory. The port had the comparison the other way
//! round: it gave the resolved directory where `$PWD` matched, and a stale `$PWD`
//! where it did not -- a session started in /a with `PWD=/b` was placed in /b. Run
//! from a folder reached through a link, three parity cases (1510, 1723, 1818) then
//! differed from tmux in the path they print.
//!
//! Each case starts a server whose first client runs with the given `$PWD`, and
//! reads the session's path back.

use std::fs;
use std::path::Path;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_kmux");

fn session_path(tmp: &Path, sock: &str, cwd: &Path, pwd: Option<&Path>) -> String {
    let mut start = Command::new(BIN);
    start
        .current_dir(cwd)
        .env("TMUX_TMPDIR", tmp)
        .env_remove("TMUX")
        .args([
            "-L",
            sock,
            "-f",
            "/dev/null",
            "new-session",
            "-d",
            "-s",
            "s",
        ]);
    match pwd {
        Some(pwd) => start.env("PWD", pwd),
        None => start.env_remove("PWD"),
    };
    assert!(
        start.status().expect("run kmux").success(),
        "no server on {sock}"
    );
    let read = Command::new(BIN)
        .env("TMUX_TMPDIR", tmp)
        .env_remove("TMUX")
        .args([
            "-L",
            sock,
            "display-message",
            "-p",
            "-t",
            "s",
            "#{session_path}",
        ])
        .output()
        .expect("run kmux");
    let _ = Command::new(BIN)
        .env("TMUX_TMPDIR", tmp)
        .args(["-L", sock, "kill-server"])
        .status();
    String::from_utf8_lossy(&read.stdout).trim_end().to_string()
}

#[test]
fn the_session_path_is_pwd_when_it_names_the_cwd_and_the_cwd_otherwise() {
    let tmp = std::env::temp_dir().join(format!("kmux-pwd-{}", std::process::id()));
    let real = tmp.join("real");
    let other = tmp.join("other");
    let link = tmp.join("link");
    fs::create_dir_all(&real).expect("make real");
    fs::create_dir_all(&other).expect("make other");
    std::os::unix::fs::symlink(&real, &link).expect("link to real");
    // The temporary folder may itself be reached through a link.
    let real_resolved = real.canonicalize().expect("resolve real");

    let cases: [(&str, Option<&Path>, &Path); 4] = [
        // $PWD names the cwd through a link: tmux keeps the link.
        ("pwd-link", Some(&link), &link),
        // $PWD names another directory: stale, so the real cwd.
        ("pwd-other", Some(&other), &real_resolved),
        // $PWD names nothing that exists: the real cwd.
        (
            "pwd-gone",
            Some(Path::new("/nonexistent-kmux-pwd")),
            &real_resolved,
        ),
        // No $PWD: the real cwd.
        ("pwd-unset", None, &real_resolved),
    ];
    let mut wrong = Vec::new();
    for (sock, pwd, want) in cases {
        let got = session_path(&tmp, sock, &real, pwd);
        if got != want.to_string_lossy() {
            wrong.push(format!(
                "{sock}: PWD={pwd:?} gave [{got}], tmux gives [{}]",
                want.display()
            ));
        }
    }
    let _ = fs::remove_dir_all(&tmp);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
