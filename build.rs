#[cfg(target_os = "windows")]
fn main() {
    if std::env::var("PROFILE").unwrap() == "release" {
        let mut res = winres::WindowsResource::new();

        res.set("ProductName", "GamePerf");
        res.set("FileDescription", "GamePerf(https://github/nzcv)");
        res.set_icon("./icon/game.ico");

        if let Err(err) = res.compile() {
            eprint!("{}", err);
            std::process::exit(1);
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn main() {}
