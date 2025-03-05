use std::env;
use std::fs::File;
use std::io::Write;
use std::process::Command;

pub fn execute(exe: &str, args: &[&str]) {
    let status = Command::new(exe)
        .args(args)
        .spawn()
        .unwrap_or_else(|_| panic!("failed to start external executable {exe}"))
        .wait()
        .expect("failed to wait on child process");

    if !status.success() {
        panic!("external executable {exe} failed with status: {}", status);
    }
}

fn get_git_hash() -> String {
    let repo = git2::Repository::open(".").unwrap();
    let head = repo.head().unwrap();
    let head_oid = head.target().unwrap();
    let head_commit = repo.find_commit(head_oid).unwrap();
    head_commit.id().to_string()
}

fn setup_version() {
    // Retrieve the Cargo version from the environment variable
    let cargo_version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "unknown".to_string());
    let commit_hash = get_git_hash();
    // Write the version to a file
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = format!("{}/version.rs", out_dir);
    let mut file = File::create(dest_path).unwrap();

    write!(
        &mut file,
        "pub const CARGO_VERSION: &str = \"{}\";\n\n pub const COMMIT_HASH: &str = \"{}\";\n",
        cargo_version, commit_hash
    )
    .unwrap();
}

fn main() {
    setup_version();
    println!("handling building assemblyscript");
    let mut pbase = env::current_dir().unwrap();
    pbase.push("samples");
    env::set_current_dir(pbase.clone()).unwrap();
    let do_asc = env::var_os("BUILD_ASC").is_some();
    if do_asc {
        execute("npm", &["install", "asc"]);
        execute("npm", &["run", "asbuild"]);
    }
    let do_fab = env::var_os("BUILD_FABRIC").is_some();
    if do_fab {
        pbase.push("go");
        pbase.push("test_wapc");

        env::set_current_dir(pbase).unwrap();
        execute(
            "tinygo",
            &[
                "build",
                "-o",
                "test_wapc.wasm",
                "-target=wasi",
                "-no-debug",
                "main.go",
            ],
        );
        execute(
            "mv",
            &[
                "-f",
                "-u",
                "test_wapc.wasm",
                "../../../target/wasm32-unknown-unknown/release/",
            ],
        )
    }
}
