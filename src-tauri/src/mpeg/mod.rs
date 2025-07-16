pub mod ffmpeg_tool;

use std::path::PathBuf;

use thiserror::Error;

#[async_trait::async_trait]
pub trait VideoEditTool {
    async fn compose_images(&self, file_list_path: &PathBuf, output: &PathBuf) -> Result<(), VideoEditToolError>;
}

#[derive(Error, Debug)]
pub enum VideoEditToolError {
    #[error("init tool failed: {0}")]
    Init(String)
}


#[async_trait::async_trait]
pub trait VoiceEditTool {
    async fn cartoned_voice(&self, input: &PathBuf, output: &PathBuf) -> Result<(), VideoEditToolError>;
}

#[derive(Error, Debug)]
pub enum VoiceEditToolError {
    #[error("Failed to handle voice: {0}")]
    Voice(String)
}


#[async_trait::async_trait]
pub trait ComposeTool {
    async fn compose_all(&self, video_input: &PathBuf, voice_input: &PathBuf, subtitle_input: &PathBuf, output: &PathBuf) -> Result<(), ComposeToolError>;
}

#[derive(Error, Debug)]
pub enum ComposeToolError {
    #[error("Failed to to compose video, voice, subtitle: {0}")]
    Fail(String)
}