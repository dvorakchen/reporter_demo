#[cfg(target_os = "windows")]
use std::path::PathBuf;
use std::{
    fs::{self, File},
    process::Command,
};

fn main() {
    get_ffmpeg_binary();
    tauri_build::build()
}

#[cfg(target_os = "windows")]
fn get_ffmpeg_binary() {
    let ffmpeg_bin = get_binary_path();
    if ffmpeg_bin.exists() {
        return;
    }

    fetch_ffmpeg(
        "https://www.gyan.dev/ffmpeg/builds/packages/ffmpeg-2025-07-07-git-d2828ab284-essentials_build.7z",
    );
}

#[cfg(target_os = "windows")]
fn fetch_ffmpeg(url: &str) {
    use tempdir::TempDir;
    let dir = TempDir::new("ffmpeg-temp").expect("Failed to  create template dir: ffmpeg-temp");

    let download_file = dir.path().join("ffmpeg-essentials_build.7z");
    let extract_to = dir.path().join("ffmpeg-essentials_build");

    if !download_file.exists() {
        let mut ffmpeg_z = File::create(&download_file).expect(&format!(
            "Failed to create file {}",
            download_file.display()
        ));

        let mut response =
            reqwest::blocking::get(url).expect(&format!("Failed to download {}", url));
        response
            .copy_to(&mut ffmpeg_z)
            .expect(&format!("Failed to download {}", url));
    }

    if !extract_to.exists() {
        sevenz_rust::decompress_file(&download_file, &extract_to)
            .expect(&format!("Failed to decompress {}", download_file.display()));
    }

    let ffmpeg_bin = get_binary_path();

    let copy_from = dir.path().join(
        "ffmpeg-essentials_build/ffmpeg-2025-07-07-git-d2828ab284-essentials_build/bin/ffmpeg.exe",
    );

    fs::create_dir_all("./binaries").expect("Create dir ./binaries failed");
    if !ffmpeg_bin.exists() {
        fs::copy(&copy_from, &ffmpeg_bin).expect(&format!(
            r#"Failed to copy filet from {} to {}"#,
            copy_from.display(),
            ffmpeg_bin.display()
        ));
    }
}

#[cfg(target_os = "windows")]
fn get_binary_path() -> PathBuf {
    let triple = get_host_triple();

    PathBuf::from(format!("./binaries/ffmpeg-{}.exe", triple))
}

fn get_host_triple() -> String {
    let output = Command::new("rustc")
        .arg("-Vv")
        .output()
        .expect("Failed to run rustc -Vv");

    let output_str = String::from_utf8(output.stdout).expect("Invalid UTF-8 in rustc -Vv");

    output_str
        .lines()
        .find(|line| line.starts_with("host: "))
        .map(|line| line.trim_start_matches("host: "))
        .expect("Failed to find host in rustc -Vv")
        .to_owned()
}
