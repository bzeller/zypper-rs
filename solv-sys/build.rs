extern crate bindgen;

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    
    let libdir_path = PathBuf::from("libsolv")
        // Canonicalize the path as `rustc-link-search` requires an absolute
        // path.
        .canonicalize()
        .expect("cannot canonicalize path");

    // our build directory
    let libdir_build_path = PathBuf::from("build")
        // Canonicalize the path as `rustc-link-search` requires an absolute
        // path.
        .canonicalize()
        .expect("cannot canonicalize path");

    // The path were we will build libsolv into
    let lib_path = libdir_build_path.join("src");
    let libext_path = libdir_build_path.join("ext");

    // Tell cargo to look for shared libraries in the specified directory
    println!("cargo:rustc-link-search={}", lib_path.to_str().unwrap());

    // Tell cargo to look for shared libraries in the specified directory
    println!("cargo:rustc-link-search={}", libext_path.to_str().unwrap());

    // Tell cargo to tell rustc to link the solv library.
    println!("cargo:rustc-link-lib=solv");

    // Tell cargo to tell rustc to link the solvext library.
    println!("cargo:rustc-link-lib=solvext");

    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=wrapper.h");

    // create builddir
    fs::create_dir_all(&libdir_build_path).expect("Failed to create builddir");

    // generate makefiles with cmake
    if !std::process::Command::new("cmake")
        .current_dir( &libdir_build_path )
        .arg("-DCMAKE_VERBOSE_MAKEFILE=TRUE")
        .arg("-DCMAKE_BUILD_TYPE=Debug")
        .arg("-DWITH_LIBXML2=1")
        .arg("-DENABLE_APPDATA=1")
        .arg("-DENABLE_COMPS=1")
        .arg("-DENABLE_STATIC=1")
        .arg("-DDISABLE_SHARED=1")
        .arg("-DENABLE_SUSEREPO=1")
        .arg("-DENABLE_HELIXREPO=1")
        .arg("-DSUSE=1")
        .arg("-DENABLE_COMPLEX_DEPS=1")
        .arg("-DUSE_VENDORDIRS=1")
        .arg("-DENABLE_BZIP2_COMPRESSION=1")
        .arg("-DENABLE_ZSTD_COMPRESSION=1")
        .arg("-DENABLE_ZCHUNK_COMPRESSION=1")
        .arg(&libdir_path)
        .output()
        .expect("could not spawn `cmake`")
        .status
        .success()
    {
        // Panic if the command was not successful.
        panic!("could not generate project files with cmake");
    }


    if !std::process::Command::new("make")
        .current_dir( &libdir_build_path )
        .arg("-j6")
        .output()
        .expect("could not spawn `make`")
        .status
        .success()
    {
        // Panic if the command was not successful.
        panic!("could not compile libsolv");
    }


    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        .clang_arg( ("-F".to_owned())+libdir_build_path.join("src").to_str().unwrap() )
        .clang_arg("-F".to_owned()+libdir_path.join("src").to_str().unwrap() )
        .clang_arg("-F".to_owned()+libdir_path.join("ext").to_str().unwrap())
        // The input header we would like to generate
        // bindings for.
        .header("wrapper.h")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
