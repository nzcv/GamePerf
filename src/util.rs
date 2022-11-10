use anyhow;
use env_logger::Env;
use log::info;
use regex::Regex;
use serde::Serialize;
use serde_json::json;
use std::{io::Write, process::Command, vec, os::windows::process::CommandExt};
use walkdir::WalkDir;

pub fn init_debug_logger() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format(|buf, record| {
            let file_line = format!("{}:{}", record.file().unwrap(), record.line().unwrap());

            writeln!(buf, "{:22} [{:05}] - {}", file_line, record.level(), record.args())
        })
        .init();
}

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn cmd(program: &str, args: String) -> anyhow::Result<(bool, String, String)> {
    let args: Vec<&str> = args.split(" ").collect();
    let output = Command::new(program).args(&args).creation_flags(CREATE_NO_WINDOW).output().unwrap();
    //command.creation_flags(CREATE_NO_WINDOW);
    // let output = command.output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "{} {:?} {} {} {}",
            program,
            args,
            output.status,
            stdout.trim(),
            stderr.trim()
        ));
    }
    Ok((output.status.success(), stdout, stderr))
}

pub fn adb(args: String) -> anyhow::Result<(bool, String, String)> {
    cmd("adb", args)
}

pub fn pid_of(package: &str) -> anyhow::Result<String> {
    let b = package.chars().all(char::is_numeric);

    if b {
        Ok(package.to_string())
    } else {
        let package = fuzzy_runing(package)?;
        Ok(package.0)
    }
}

pub fn fuzzy_runing(s: &str) -> anyhow::Result<(String, String)> {
    let args = format!("shell ps -e");
    let (succeed, stdout, _e) = adb(args)?;
    if succeed {
        for l in stdout.lines() {
            let items: Vec<&str> = l.trim().split_whitespace().collect();

            if items.last().unwrap().contains(s) {
                let package = *items.last().unwrap();
                let pid = items[1].to_string();
                return Ok((pid, package.to_string()));
            }
        }
    }
    Err(anyhow::anyhow!("Not Found {}", s))
}

pub fn fuzzy_package(s: &str) -> anyhow::Result<String> {
    let args = format!("shell pm list packages -f {}", s);
    let (succeed, stdout, _e) = adb(args)?;
    if succeed {
        for line in stdout.lines() {
            if line.contains(s) {
                let p = line.split("=").last().unwrap();
                log::info!("{}", line,);
                return Ok(format!("{}", p));
            }
        }
    }
    Err(anyhow::anyhow!("Not Found {}", s))
}

pub fn check_inject_so_succeed(package: &str, pid: &str, library: &str) -> anyhow::Result<bool> {
    let (_, stdout, _) = adb(format!("shell run-as {} cat /proc/{}/maps", package, pid))?;
    let mut lines = stdout.lines();
    while let Some(line) = lines.next() {
        if line.ends_with(library) {
            info!("[*] {}", line);
            return Ok(true);
        }
    }
    Err(anyhow::anyhow!("Error Inject {}", library))
}

pub fn get_built_file(file: &str) -> Option<String> {
    find_file(file, "built")
}

pub fn find_file(file: &str, root: &str) -> Option<String> {
    let mut files = vec![];

    for entry in WalkDir::new(root) {
        let entry = entry.unwrap();

        if entry.file_type().is_file() {
            files.push((
                format!("{}", entry.path().to_str().unwrap()),
                format!("{}", entry.file_name().to_str().unwrap()),
            ));
        }
    }

    for (path, filename) in files {
        if path.contains(file) {
            info!("{} {}", path, filename);
            return Some(path);
        }
    }
    None
}

pub fn get_android_prop(key: &str) -> anyhow::Result<String> {
    let (_, stdout, _) = adb(format!("shell getprop"))?;
    let mut lines = stdout.lines();
    while let Some(line) = lines.next() {
        if line.contains(key) {
            let re = Regex::new(r"\[(.*)\]: \[(.*)\]").unwrap();
            let cap = re.captures(line);
            let val = cap.unwrap().get(2).map_or("", |m| m.as_str());
            // info!("get_android_prop: {:?}", ret);
            //let items = line.split(' ');
            // let v = items.last().unwrap();
            // info!("[*] {}", line);
            return Ok(val.to_string());
        }
    }
    Err(anyhow::anyhow!("prop error"))
}

enum ParserSection {
    HEADER,
    PSSINFO,
    APPSUMMARY,
    OBJECTS,
    SQL,
    DATABASES,
}

#[derive(Debug, Serialize)]
pub struct PssInfo {
    package_name: String,
    pid: String,
    pss_header: Vec<String>,
    pss_values: Vec<u64>,
    cursor: u64,
    app_header: Vec<String>,
    app_values: Vec<u64>,
}
static mut index: u64 = 0;

impl PssInfo {
    pub fn new() -> PssInfo {
        PssInfo {
            package_name: "".into(),
            pid: "".into(),
            pss_header: vec!["index".to_string()],
            pss_values: vec![],
            cursor: 0,
			app_header: vec!["index".to_string()],
			app_values: vec![],
        }
    }

    pub fn inc_index(&mut self) {
        unsafe {
            self.cursor = index;
            index += 1;
        }
        self.pss_values.push(self.cursor);
		self.app_values.push(self.cursor);
    }
}

pub fn dump_pss(package_name: &str) -> anyhow::Result<PssInfo> {
    log::info!("dump pss {}", package_name);
    let (_, stdout, _) = adb(format!("shell dumpsys meminfo {}", package_name))?;
    let mut lines = stdout.lines();
    let mut parser_status = ParserSection::HEADER;
    let mut pss_data = PssInfo::new();
    let re_app_info = Regex::new(r"\*\* MEMINFO in pid (\d+) \[(.*)\] \*\*").unwrap();
    let re_pss_info = Regex::new(r"(\D+)(\d+) .*").unwrap();
    let re_summary_info = Regex::new(r"(\D+):\s+(\d+)").unwrap();
    while let Some(line) = lines.next() {
        let line = line.trim();
        match parser_status {
            ParserSection::HEADER => {
                if line.contains("------") {
                    parser_status = ParserSection::PSSINFO;
                    continue;
                }
                let cap = re_app_info.captures(line);
                if let Some(cap) = cap {
                    let package_name = cap.get(1).unwrap().as_str();
                    let pid = cap.get(2).unwrap().as_str();
                    pss_data.package_name = package_name.into();
                    pss_data.pid = pid.into();
                    pss_data.inc_index();
                    log::debug!("package_name={}, pid={}", package_name, pid);
                }
            }
            ParserSection::PSSINFO => {
                if line.contains("App Summary") {
                    parser_status = ParserSection::APPSUMMARY;
                    log::debug!("App Summary:");
                    continue;
                }
                let cap = re_pss_info.captures(line);
                if let Some(cap) = cap {
                    let name = cap.get(1).unwrap().as_str().trim();
                    let val = cap.get(2).unwrap().as_str();
                    pss_data.pss_header.push(name.into());
                    pss_data.pss_values.push(val.parse::<u64>().unwrap() / 1024);
                    log::debug!("name:{}, val:{}", name, val);
                }
            }
            ParserSection::APPSUMMARY => {
                if line.contains("Objects") {
                    parser_status = ParserSection::OBJECTS;
                    log::debug!("Objects:");
                    continue;
                }
                let cap = re_summary_info.captures(line);
                if let Some(cap) = cap {
                    let name = cap.get(1).unwrap().as_str().trim();
                    let val = cap.get(2).unwrap().as_str();
                    pss_data.app_header.push(name.into());
                    pss_data.app_values.push(val.parse::<u64>().unwrap() / 1024);
                    log::debug!("{}\t{}", name, val);
                }
            }
            ParserSection::OBJECTS => {}
            ParserSection::SQL => {}
            ParserSection::DATABASES => {}
        }
    }
    return Ok(pss_data);
    // Err(anyhow::anyhow!("dump pss error"))
}


pub fn current_app() -> anyhow::Result<String> {
    let out = adb(format!("shell dumpsys activity activities|grep mResumedActivity"))?;        

    // let err = String::from_utf8(out.stderr).unwrap();
    // if err.len() > 0 {
    //     // return Err(Box::new(UnExpectedError::Custom(format!("Adb Error"))));
    //     return Err(anyhow::anyhow!("Adb Error"));
    // }

    //let p = String::from_utf8(out.1).unwrap();

    let re = Regex::new(r"com.[a-z0-9A-Z]*.[a-z0-9A-Z.]*").unwrap();

    let cap = re.captures(out.1.as_str());

    let ret = cap.unwrap().get(0).map_or("", |m| m.as_str());

    Ok(format!("{}", ret))
}