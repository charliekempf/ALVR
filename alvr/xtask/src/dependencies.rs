use crate::{
    command::{self, run_as_bash_in as bash_in},
    workspace_dir,
};
use std::{
    fs,
    io::ErrorKind,
    panic,
    path::{Path, PathBuf},
};

fn deps_dir() -> PathBuf {
    workspace_dir().join("deps")
}

fn download_and_extract_zip(url: &str, destination: &Path) {
    let zip_file = deps_dir().join("temp_download.zip");

    fs::remove_file(&zip_file).ok();
    fs::create_dir_all(deps_dir()).unwrap();
    command::download(url, &zip_file).unwrap();

    fs::remove_dir_all(&destination).ok();
    fs::create_dir_all(&destination).unwrap();
    command::unzip(&zip_file, &destination).unwrap();

    fs::remove_file(zip_file).unwrap();
}

fn build_rust_android_gradle() {
    const PLUGIN_COMMIT: &str = "6e553c13ef2d9bb40b58a7675b96e0757d1b0443";
    const PLUGIN_VERSION: &str = "0.8.3";

    let temp_build_dir = deps_dir().join("temp_gradle_plugin_build");
    download_and_extract_zip(
        &format!(
            "https://codeload.github.com/mozilla/rust-android-gradle/zip/{}",
            PLUGIN_COMMIT
        ),
        &temp_build_dir,
    );
    let download_path = temp_build_dir.join(format!("rust-android-gradle-{}", PLUGIN_COMMIT));

    #[cfg(windows)]
    let gradlew_path = download_path.join("gradlew.bat");
    #[cfg(target_os = "linux")]
    let gradlew_path = download_path.join("gradlew");

    command::run_in(
        &download_path,
        &format!("{} publish", gradlew_path.to_string_lossy()),
    )
    .unwrap();

    let dep_dir = crate::workspace_dir()
        .join("deps")
        .join("rust-android-gradle");
    if let Err(e) = fs::create_dir_all(&dep_dir) {
        if e.kind() != ErrorKind::AlreadyExists {
            panic::panic_any(e);
        }
    }

    // Workaround for long path issue on Windows - canonicalize
    let plugin_path = download_path.canonicalize().unwrap();
    let plugin_path = plugin_path
        .join("samples")
        .join("maven-repo")
        .join("org")
        .join("mozilla")
        .join("rust-android-gradle")
        .join("rust-android")
        .join(PLUGIN_VERSION)
        .join(format!("rust-android-{}.jar", PLUGIN_VERSION));
    fs::copy(
        plugin_path,
        dep_dir.join(format!("rust-android-{}.jar", PLUGIN_VERSION)),
    )
    .unwrap();

    fs::remove_dir_all(temp_build_dir).ok();
}

pub fn build_ffmpeg_linux() {
    // dependencies: build-essential pkg-config nasm libva-dev libvulkan-dev libx264-dev libx265-dev

    let download_path = deps_dir().join("ubuntu");
    download_and_extract_zip(
        "https://codeload.github.com/FFmpeg/FFmpeg/zip/n4.4",
        &download_path,
    );
    let ffmpeg_path = download_path.join("FFmpeg-n4.4");

    bash_in(
        &ffmpeg_path,
        &format!(
            "./configure {} {} {} {} {} {} {} {} {}",
            "--enable-gpl --enable-version3",
            "--disable-static --enable-shared",
            "--disable-programs",
            "--disable-doc",
            "--disable-avdevice --disable-avformat --disable-swresample --disable-postproc",
            "--disable-network",
            "--enable-lto",
            format!(
                "--disable-everything {} {} {} {}",
                "--enable-encoder=h264_vaapi --enable-encoder=hevc_vaapi",
                "--enable-encoder=libx264 --enable-encoder=libx264rgb --enable-encoder=libx265",
                "--enable-hwaccel=h264_vaapi --enable-hwaccel=hevc_vaapi",
                "--enable-filter=scale --enable-filter=scale_vaapi",
            ),
            "--enable-libx264 --enable-libx265 --enable-vulkan",
        ),
    )
    .unwrap();
    bash_in(&ffmpeg_path, "make -j$(nproc)").unwrap();
}

pub fn build_deps(target_os: &str) {
    if target_os == "windows" {
        command::run("cargo install wasm-pack").unwrap();
    } else if target_os == "android" {
        command::run("rustup target add aarch64-linux-android").unwrap();
        build_rust_android_gradle();
    } else {
        println!("Nothing to do for {}!", target_os)
    }
}
