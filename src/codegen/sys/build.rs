use super::collect_versions;
use crate::{codegen::general, env::Env, file_saver::save_to_file, library::MAIN_NAMESPACE};
use log::info;
use std::io::{Result, Write};

pub fn generate(env: &Env) {
    info!(
        "Generating sys build script for {}",
        env.config.library_name
    );

    let split_build_rs = env.config.split_build_rs;
    let path = env.config.target_path.join("build.rs");

    if !split_build_rs || !path.exists() {
        info!("Generating file {:?}", path);
        save_to_file(&path, env.config.make_backup, |w| {
            generate_build_script(w, env, split_build_rs)
        });
    }

    if split_build_rs {
        let path = env.config.target_path.join("build_version.rs");
        info!("Generating file {:?}", path);
        save_to_file(&path, env.config.make_backup, |w| {
            generate_build_version(w, env)
        });
    }
}

#[allow(clippy::write_literal)]
fn generate_build_script(w: &mut dyn Write, env: &Env, split_build_rs: bool) -> Result<()> {
    let ns = env.library.namespace(MAIN_NAMESPACE);
    let package_name = ns.package_name.as_ref().expect("Missing package name");

    if !split_build_rs {
        general::start_comments(w, &env.config)?;
        writeln!(w)?;
    }

    writeln!(w, "use std::env;")?;
    writeln!(
        w,
        "{}",
        r##"#[cfg(not(feature = "dox"))]
use std::process;"##
    )?;

    if split_build_rs {
        writeln!(w)?;
        writeln!(w, "mod build_version;")?;
    }

    write!(
        w,
        "{}",
        r##"
#[cfg(feature = "dox")]
fn main() {} // prevent linking libraries to avoid documentation failure

#[cfg(not(feature = "dox"))]
fn main() {
    let libs = system_deps::Config::new().probe();
    if let Err(s) = libs {
        println!("cargo:warning={}", s);
        process::exit(1);
    }

    let libs = libs.unwrap();

    if env::var("CARGO_FEATURE_ABI_TESTS").is_ok() {
"##
    )?;

    write!(
        w,
        "        let includes = libs.get(\"{}\").unwrap().include_paths.clone();",
        package_name
    )?;

    write!(
        w,
        "{}",
        r##"

        let mut cc = cc::Build::new();

        cc.flag_if_supported("-Wno-deprecated-declarations");
        cc.flag_if_supported("-std=c11"); // for _Generic

        cc.file("tests/constant.c");
        cc.file("tests/layout.c");

        for i in includes {
            cc.include(i);
        }

        cc.compile("cabitests");
    }
}
"##
    )
}

fn generate_build_version(w: &mut dyn Write, env: &Env) -> Result<()> {
    general::start_comments(w, &env.config)?;
    writeln!(w)?;
    writeln!(w, "pub fn version() -> &'static str {{")?;
    write_version(w, env, false)?;
    writeln!(w, "}}")
}

fn write_version(w: &mut dyn Write, env: &Env, for_let: bool) -> Result<()> {
    let versions = collect_versions(env);

    for (version, lib_version) in versions.iter().rev() {
        write!(
            w,
            "if cfg!({}) {{\n\t\t\"{}\"\n\t}} else ",
            version.to_cfg(),
            lib_version
        )?;
    }
    let end = if for_let { ";" } else { "" };
    writeln!(w, "{{\n\t\t\"{}\"\n\t}}{}", env.config.min_cfg_version, end)
}
