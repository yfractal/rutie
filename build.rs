extern crate pkg_config;
use std::env;
use std::ffi::OsStr;
use std::process::Command;
use std::path::{Path};

fn rbconfig(key: &str) -> String {
    let ruby = match env::var_os("RUBY") {
        Some(val) => val.to_os_string(),
        None => OsStr::new("ruby").to_os_string(),
    };
    let config = Command::new(ruby)
        .arg("-e")
        .arg(format!("print RbConfig::CONFIG['{}']", key))
        .output()
        .unwrap_or_else(|e| panic!("ruby not found: {}", e));

    String::from_utf8(config.stdout).expect("RbConfig value not UTF-8!")
}

fn set_env_pkg_config() {
    let key = "PKG_CONFIG_PATH";
    let value = Path::new(&rbconfig("libdir")).join("pkgconfig");
    std::env::set_var(key, value);
}

fn trim_teeny(version: &str) -> &str {
    version.rsplitn(2, '.').collect::<Vec<&str>>().last().unwrap()
}

fn ruby_version() -> String {
    rbconfig("RUBY_PROGRAM_VERSION")
}

fn use_libdir() {
    println!("cargo:rustc-link-search={}", rbconfig("libdir"));
}

fn transform_lib_args(rbconfig_key: &str, replacement: &str) -> String {
    rbconfig(rbconfig_key).replace("-l", replacement)
}

fn use_static() {
    // Ruby gives back the libs in the form: `-lpthread -lgmp`
    // Cargo wants them as: `-l pthread -l gmp`
    println!("cargo:rustc-flags={}", transform_lib_args("LIBS", "-l "));
}

fn use_dylib() {
    use_libdir();
    println!("cargo:rustc-link-lib=dylib={}", rbconfig("RUBY_SO_NAME"));
}

fn main() {
    // Ruby programs calling Rust don't need cc linking
    if let None = std::env::var_os("NO_LINK_RUTIE") {
        // Ruby includes pkgconfig under their lib dir
        set_env_pkg_config();

        match pkg_config::Config::new().atleast_version(trim_teeny(&ruby_version())).probe("ruby") {
            Ok(_) => return,
            Err(err) => eprintln!("{:?}", err),
        }
        
        if rbconfig("target_os") != "mingw32" && env::var_os("RUBY_STATIC").is_some() {
            use_static()
        } else {
            match rbconfig("ENABLE_SHARED").as_str() {
                "no" => use_static(),
                "yes" => use_dylib(),
                _ => {
                    let msg = "Error! Couldn't find a valid value for \
                    RbConfig::CONFIG['ENABLE_SHARED']. \
                    This may mean that your ruby's build config is corrupted. \
                    Possible solution: build a new Ruby with the `--enable-shared` configure opt.";
                    panic!(msg)
                }
            }
        }
    }
}
