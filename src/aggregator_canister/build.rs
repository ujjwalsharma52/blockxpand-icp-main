use std::env;
use std::process::Command;

fn main() {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("git command");
    let sha = String::from_utf8(output.stdout).expect("utf8");
    let sha = sha.trim();
    if let Ok(env_sha) = env::var("GIT_SHA") {
        if env_sha != sha {
            panic!("GIT_SHA mismatch: {env_sha} != {sha}");
        }
    }
    println!("cargo:rustc-env=GIT_SHA={sha}");
    let time = chrono::Utc::now().to_rfc3339();
    println!("cargo:rustc-env=BUILD_TIME={time}");
}
