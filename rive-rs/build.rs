use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
};

use walkdir::WalkDir;

fn all_files_with_extension<P: AsRef<Path>>(
    path: P,
    extension: &str,
) -> impl Iterator<Item = PathBuf> + '_ {
    WalkDir::new(path).into_iter().filter_map(move |entry| {
        entry
            .ok()
            .map(|entry| entry.into_path())
            .filter(|path| path.extension() == Some(&OsString::from(extension)))
    })
}

fn main() {
    println!("cargo:rerun-if-changed=src/ffi.cpp");

    let rive_cpp_path = env::var("RIVE_CPP_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("../submodules/rive-cpp"));

    let is_msvc = cfg!(target_env = "msvc");

    let cpp_standard_flag = if is_msvc {
        "/std:c++14"
    } else {
        "-std=c++14"
    };

    // Set the exception handling flag for MSVC
    let eh_flag = if is_msvc {
        "/EHsc"
    } else {
        ""
    };

    // Compile ffi.cpp
    cc::Build::new()
        .cpp(true)
        .include(rive_cpp_path.join("include"))
        .file("src/ffi.cpp")
        .flag(cpp_standard_flag)
        .flag(eh_flag)
        .warnings(false)
        .compile("rive-ffi");

    // Compile Yoga if the 'layout' feature is enabled
    if cfg!(feature = "layout") {
        let layout_flag = if is_msvc {
            "/std:c++11"
        } else {
            "-std=c++11"
        };
        let layout_eh_flag = if is_msvc {
            "/EHsc"
        } else {
            ""
        };

        cc::Build::new()
            .cpp(true)
            .flag(layout_flag)
            .flag(layout_eh_flag)
            .files(all_files_with_extension("../submodules/yoga/yoga", "cpp"))
            .include("../submodules/yoga")
            .define("YOGA_EXPORT=", None)
            .warnings(false)
            .compile("yoga");
    }

    // Compile HarfBuzz and SheenBidi if the 'text' feature is enabled
    if cfg!(feature = "text") {
        let target = env::var("TARGET").unwrap();
        let profile = env::var("PROFILE").unwrap();

        let mut cfg_build = cc::Build::new();
        cfg_build
            .cpp(true)
            .flag_if_supported(if is_msvc { "/std:c++11" } else { "-std=c++11" })
            .flag_if_supported(if is_msvc { "/EHsc" } else { "" })
            .warnings(false)
            .file("../submodules/harfbuzz/src/harfbuzz.cc");

        if !target.contains("windows") {
            cfg_build.define("HAVE_PTHREAD", "1");
        }

        if target.contains("apple") && profile.contains("release") {
            cfg_build.define("HAVE_CORETEXT", "1");
        }

        if target.contains("windows") {
            cfg_build.define("HAVE_DIRECTWRITE", "1");
        }

        if target.contains("windows-gnu") {
            cfg_build.flag("-Wa,-mbig-obj");
        }

        cfg_build.compile("harfbuzz");

        cc::Build::new()
            .files(all_files_with_extension(
                "../submodules/SheenBidi/Source",
                "c",
            ))
            .include("../submodules/SheenBidi/Headers")
            .flag_if_supported(if is_msvc { "/EHsc" } else { "" })
            .warnings(false)
            .compile("sheenbidi");
    }

    // Compile the main Rive library
    let mut cfg_final = cc::Build::new();
    cfg_final
        .cpp(true)
        .include(rive_cpp_path.join("include"))
        .files(all_files_with_extension(rive_cpp_path.join("src"), "cpp"))
        .flag(cpp_standard_flag)
        .flag(eh_flag)
        .define("_RIVE_INTERNAL_", None)
        .warnings(false);

    if cfg!(feature = "text") {
        cfg_final
            .include("../submodules/harfbuzz/src")
            .include("../submodules/SheenBidi/Headers")
            .flag_if_supported(if is_msvc {
                "/Wno-deprecated-declarations"
            } else {
                "-Wno-deprecated-declarations"
            })
            .define("WITH_RIVE_TEXT", None);
    }
    if cfg!(feature = "layout") {
        cfg_final
            .include("../submodules/yoga")
            .flag_if_supported(if is_msvc {
                "/Wno-deprecated-declarations"
            } else {
                "-Wno-deprecated-declarations"
            })
            .define("WITH_RIVE_LAYOUT", None)
            .define("YOGA_EXPORT=", None);
    }

    cfg_final.compile("rive");
}
