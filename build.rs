extern crate shlex;

use std::env;

fn main() {
    let cflags = env::var("DEP_RIOT_SYS_CFLAGS")
        .expect("DEP_RIOT_SYS_CFLAGS is not set, check whether riot-sys exports it.");
    let cflags = shlex::split(&cflags).expect("Odd shell escaping in CFLAGS");

    // riot-sys can pass used modules in DEP_RIOT_SYS_USEMODULE
    // if it does, handle those here.
    // alternatively, parse "-DMODULE_" below.
    if let Some(modules) = env::var("DEP_RIOT_SYS_USEMODULE").ok() {
        let modules = shlex::split(&modules).expect("Odd shell escaping in USEMODULE");
        for module in &modules {
            println!("cargo:rustc-cfg=riot_module_{}", module);
        }
    }

    println!("cargo:rerun-if-env-changed=DEP_RIOT_SYS_CFLAGS");
    println!("cargo:rerun-if-env-changed=DEP_RIOT_SYS_USEMODULE");

    for flag in cflags.iter() {
        if flag.starts_with("-DMODULE_") {
            // Some modules like cmsis-dsp_StatisticsFunctions have funny characters
            println!(
                "cargo:rustc-cfg=riot_module_{}",
                flag[9..].to_lowercase().replace("-", "_")
            );
        }

        if flag == "-DDEVELHELP" {
            println!("cargo:rustc-cfg=riot_develhelp");
        }
    }

    let mut bindgen_output_file = None;

    for (key, value) in env::vars() {
        if let Some(marker) = key.strip_prefix("DEP_RIOT_SYS_MARKER_") {
            println!("cargo:rerun-if-env-changed={}", key);
            // It appears that they get uppercased in Cargo -- but should be lower-case as in the
            // original riot-sys build.rs, especially to not make the cfg statements look weird.
            println!("cargo:rustc-cfg=marker_{}", marker.to_lowercase());
        }
        if key == "DEP_RIOT_SYS_BINDGEN_OUTPUT_FILE" {
            bindgen_output_file = Some(value);
        }
    }

    if let Some(bindgen_output_file) = bindgen_output_file {
        let bindgen_output = std::fs::read_to_string(bindgen_output_file)
            .expect("Failed to read BINDGEN_OUTPUT_FILE");

        // Whether or not the extra space is there depends on whether or not rustfmt is installed.
        // FIXME: Find a better way to extract that information
        if bindgen_output.contains("pub const CONFIG_AUTO_INIT_ENABLE_DEBUG: u32 = 1;")
            || bindgen_output.contains("pub const CONFIG_AUTO_INIT_ENABLE_DEBUG : u32 = 1 ;")
        {
            println!("cargo:rustc-cfg=marker_config_auto_init_enable_debug");
        }
    } else {
        println!("cargo:warning=Old riot-sys did not provide BINDGEN_OUTPUT_FILE, assuming it's an old RIOT version");
    }
}
