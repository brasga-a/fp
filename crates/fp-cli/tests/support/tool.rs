//! Executável de teste nativo; compilado pelo rustc, sem ferramentas de ecossistema.
use std::{env, fs, io::{self, Read, Write}, path::PathBuf};
fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    let exe=env::current_exe().unwrap();
    let name=exe.file_stem().unwrap().to_string_lossy();
    if name == "fail" { std::process::exit(37); }
    if name == "docker" {
        let mut log=fs::OpenOptions::new().create(true).append(true).open(exe.with_extension("calls")).unwrap();
        writeln!(log,"{}|context={}|host={}",args.join(" "),env::var("DOCKER_CONTEXT").unwrap_or_default(),env::var("DOCKER_HOST").unwrap_or_default()).unwrap();
        match args.first().map(String::as_str) {
            Some("info") if args.iter().any(|a|a=="--format") => println!("linux"),
            Some("info"|"ps") => {},
            Some("volume") => match args.get(1).map(String::as_str) {
                Some("create") => println!("fp-fixture-data"),
                Some("inspect") => println!(r#"[{{"Labels":{{"fp.resource":"fixture"}}}}]"#),
                _ => {},
            },
            Some("create") => {
                assert!(!env::var("MONGO_INITDB_ROOT_PASSWORD").unwrap().is_empty());
                println!("fixture-id");
            }
            Some("container") => println!(r#"[{{"Id":"fixture-id","State":{{"Running":false}},"Config":{{"Image":"mongo:8.0"}},"Mounts":[{{"Name":"fp-fixture-data"}}]}}]"#),
            Some("exec") => { let mut input=String::new(); io::stdin().read_to_string(&mut input).unwrap(); },
            Some("rm") => {},
            _ => std::process::exit(41),
        }
        return;
    }
    fs::write("arguments.log", args.join("\n")).unwrap();
    match name.as_ref() {
        "uv" => {
            fs::write("pyproject.toml", "[project]\nname='fixture'\nversion='0.0.0'\n").unwrap();
            fs::write("main.py", "print('fixture')\n").unwrap();
        }
        "cargo" => {
            fs::create_dir("src").unwrap();
            fs::write("src/main.rs", "fn main() {}\n").unwrap();
            fs::write("Cargo.toml", "[package]\nname='fixture'\nversion='0.0.0'\n").unwrap();
        }
        "vp" => { fs::write("package.json", r#"{"scripts":{"dev":"fixture"}}"#).unwrap(); }
        "pio" => { fs::write("platformio.ini", "[env:fixture]\n").unwrap(); }
        _ => {
            println!("{}", env::current_dir().unwrap().display());
            for arg in &args { println!("{arg}"); }
            if args.iter().any(|arg|arg=="--stdin") {
                let mut input=String::new(); io::stdin().read_to_string(&mut input).unwrap();
                print!("{input}"); io::stdout().flush().unwrap();
            }
            eprintln!("fixture-stderr");
        }
    }
    let change = exe.with_extension("registry");
    if change.exists() {
        let path=PathBuf::from(fs::read_to_string(change).unwrap());
        fs::OpenOptions::new().append(true).open(path).unwrap().write_all(b"\n# external change\n").unwrap();
    }
    if args.iter().any(|arg|arg=="exit37") { std::process::exit(37); }
}
