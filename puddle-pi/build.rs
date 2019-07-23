fn main() {
    let date = chrono::offset::Local::now().format("%F_%T");
    let sha = std::process::Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .map(|out| String::from_utf8(out.stdout).unwrap())
        .unwrap_or_else(|_| "no git".to_owned());
    println!("cargo:rustc-env=PI_TEST_ABOUT={}_{}", date, sha);
}
