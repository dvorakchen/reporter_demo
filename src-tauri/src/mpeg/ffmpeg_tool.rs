use std::path::PathBuf;

use tauri::AppHandle;
use tauri_plugin_shell::{
    ShellExt,
    process::{CommandEvent, TerminatedPayload},
};

use crate::mpeg::{
    ComposeTool, ComposeToolError, VideoEditTool, VideoEditToolError, VoiceEditTool,
};

pub struct FFmpeg4Video(pub AppHandle);

#[async_trait::async_trait]
impl VideoEditTool for FFmpeg4Video {
    async fn compose_images(
        &self,
        file_list_path: &PathBuf,
        output: &PathBuf,
    ) -> Result<(), VideoEditToolError> {
        let ffmpeg = self.0.shell().sidecar("ffmpeg").map_err(|_| VideoEditToolError::Init("launching ffmpeg failed".to_owned()))?
        .arg("-y")
            .arg("-f")
            .arg("concat")
            .arg("-i")
            .arg(&file_list_path)
            .arg("-vf")
            .arg("scale=720:1280:force_original_aspect_ratio=decrease,pad=720:1280:(ow-iw)/2:(oh-ih)/2:color=black")
            .arg("-c:v")
            .arg("libx264")
            .arg("-r")
            .arg("30")
            .arg("-pix_fmt")
            .arg("yuv420p")
            .arg(output);

        let (mut rx, mut _child) = ffmpeg.spawn().expect("Failed to spawn sidecar");
        let join = tauri::async_runtime::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    CommandEvent::Stdout(line) => {
                        // log
                        print!("{}", String::from_utf8(line).unwrap());
                    }
                    CommandEvent::Stderr(line) => {
                        // log
                        print!("{}", String::from_utf8(line).unwrap());
                    }
                    CommandEvent::Error(line) => {
                        // log
                        print!("{}", line);
                    }
                    CommandEvent::Terminated(TerminatedPayload { code: Some(v), .. }) if v != 0 => {
                        // log
                        print!("{}", v);
                    }
                    _ => {}
                }
            }
        });
        join.await.expect("ffmpeg handle video error");

        Ok(())
    }
}

pub struct FFmpeg4Voice(pub AppHandle);

#[async_trait::async_trait]
impl VoiceEditTool for FFmpeg4Voice {
    async fn cartoned_voice(
        &self,
        input: &PathBuf,
        output: &PathBuf,
    ) -> Result<(), VideoEditToolError> {
        let ffmpeg = self
            .0
            .shell()
            .sidecar("ffmpeg")
            .expect("Failed to get sidecar: ffmpeg")
            .args([
                "-i",
                input.to_str().unwrap(),
                "-af",
                "asetrate=30000, aresample=22050, atempo=1",
                output.to_str().unwrap(),
            ]);

        let (mut rx, mut _child) = ffmpeg.spawn().expect("Failed to spawn sidecar");
        let join = tauri::async_runtime::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    CommandEvent::Stdout(_line) => {
                        // log
                    }
                    CommandEvent::Stderr(_line) => {
                        // log
                    }
                    CommandEvent::Error(_line) => {
                        // log
                    }
                    CommandEvent::Terminated(TerminatedPayload { code: Some(v), .. }) if v != 0 => {
                        // log
                    }
                    _ => {}
                }
            }
        });

        join.await.expect("ffmpeg handle voice error");

        Ok(())
    }
}

pub struct FFmpeg4Compose(pub AppHandle);

#[async_trait::async_trait]
impl ComposeTool for FFmpeg4Compose {
    async fn compose_all(
        &self,
        video_input: &PathBuf,
        voice_input: &PathBuf,
        subtitle_input: &PathBuf,
        output: &PathBuf,
    ) -> Result<(), ComposeToolError> {
        let s = subtitle_input.display().to_string().replace('\\', "/");
        let ffmpeg = self
            .0
            .shell()
            .sidecar("ffmpeg")
            .expect("Failed to get sidecar: ffmpeg")
            .arg("-i")
            .arg(video_input)
            .arg("-i")
            .arg(voice_input)
            .arg("-vf")
            .arg(&format!("subtitles='{}'", s))
            .arg("-c:v")
            .arg("libx264")
            .arg("-c:a")
            .arg("aac")
            .arg("-b:a")
            .arg("192k")
            .arg("-shortest")
            .arg("-y")
            .arg(output);

        let (mut rx, mut _child) = ffmpeg.spawn().expect("Failed to spawn sidecar");
        let join = tauri::async_runtime::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    CommandEvent::Stdout(line) => {
                        // log
                        print!("{}", String::from_utf8(line).unwrap());
                    }
                    CommandEvent::Stderr(line) => {
                        // log
                        print!("{}", String::from_utf8(line).unwrap());
                    }
                    CommandEvent::Error(line) => {
                        // log
                        print!("{}", line);
                    }
                    CommandEvent::Terminated(TerminatedPayload { code: Some(v), .. }) if v != 0 => {
                        // log
                        print!("{}", v);
                    }
                    _ => {}
                }
            }
        });
        join.await.expect("ffmpeg handle compose error");

        Ok(())
    }
}
