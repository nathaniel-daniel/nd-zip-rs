use std::process::Command;

fn get_git_rev() -> String {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("failed to spawn git");
    let mut stdout = String::from_utf8(output.stdout).expect("stdout is not utf8");
    if stdout.ends_with("\n") {
        stdout.pop();
    }
    if stdout.ends_with("\r") {
        stdout.pop();
    }
    let stderr = String::from_utf8(output.stderr).expect("stderr is not utf8");

    if !output.status.success() {
        panic!("invalid status code {0}: {stderr}", output.status);
    }

    stdout
}

fn main() {
    println!("cargo::rerun-if-changed=.git/HEAD");
    let head_content = std::fs::read_to_string(".git/HEAD").expect("failed to read git head file");
    if let Some(git_ref) = head_content
        .strip_prefix("ref: ")
        .map(|git_ref| git_ref.trim())
    {
        let git_ref_path = format!(".git/{git_ref}");
        println!("cargo::rerun-if-changed={git_ref_path}");
    }

    println!("cargo::rustc-env=GIT_REV={}", get_git_rev());
}
