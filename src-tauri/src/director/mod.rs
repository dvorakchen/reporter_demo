pub mod source;
use std::{io, path::PathBuf, time::Duration};

use hound::WavReader;
use nanoid::nanoid;
use tauri::AppHandle;
use thiserror::Error;
use tokio::fs;

use crate::{
    director::source::{NewsSource, SourceName},
    mpeg::{
        ComposeTool, ComposeToolError, VideoEditToolError, VoiceEditTool, VoiceEditToolError,
        ffmpeg_tool::{FFmpeg4Compose, FFmpeg4Video, FFmpeg4Voice},
    },
    news::{NewsMaterial, NewsMaterialError, NewsTitle},
    subtitle::{SingleSubtitle, Subtitle, SubtitleError, srt::SrtSubtitle},
    tts::{TTSError, TTSFile, TTSService, ali_tts::AliTTS, get_wav_len},
    video::{VideoEditor, VideoEditorError, junior_editor::JuniorEditor},
};

#[derive(Error, Debug)]
pub enum DirectorError {
    #[error("fail occurred: {0}")]
    Source(String),
    #[error("get material failed: {0}")]
    Material(#[from] NewsMaterialError),
    #[error("tts error: {0}")]
    TTS(#[from] TTSError),
    #[error("subtitle error: {0}")]
    Subtitle(#[from] SubtitleError),
    #[error("file error")]
    IO(#[from] io::Error),
    #[error("video editor error: {0}")]
    VideoEditor(#[from] VideoEditorError),
    #[error("video editor tool error: {0}")]
    VideoEditorTool(#[from] VideoEditToolError),
    #[error("voice editor tool error: {0}")]
    VoiceEditorTool(#[from] VoiceEditToolError),
    #[error("compose tool error: {0}")]
    ComposeTool(#[from] ComposeToolError),
    #[error("wav error: {0}")]
    WavReader(#[from] hound::Error),
}

pub struct NewsShortVideo {
    pub title: String,
    pub path: PathBuf,
}

pub type DirectorResult<T> = Result<T, DirectorError>;

pub struct Director {
    sources: Vec<NewsSource>,
    tts: Option<Box<dyn TTSService + Sync + Send + 'static>>,
    subtitle: Option<Box<dyn Subtitle + Sync + Send + 'static>>,
    video_editor: Option<Box<dyn VideoEditor + Sync + Send + 'static>>,
    voice_edit_tool: Option<Box<dyn VoiceEditTool + Sync + Send + 'static>>,
    compose_tool: Option<Box<dyn ComposeTool + Sync + Send + 'static>>,
}

impl Director {
    pub fn default(
        tts_url: String,
        ali_key: String,
        deepseek_api_key: String,
        app: AppHandle,
    ) -> Self {
        let tts = AliTTS::new(tts_url, ali_key);
        let subtitle = SrtSubtitle::new();
        let video_editor = JuniorEditor::new(FFmpeg4Video(app.clone()));
        let voice_edit_tool = FFmpeg4Voice(app.clone());
        let compose_tool = FFmpeg4Compose(app.clone());

        Self::new(deepseek_api_key)
            .with_tts(tts)
            .with_subtitle(subtitle)
            .with_video_editor(video_editor)
            .with_voice_edit_tool(voice_edit_tool)
            .with_compose_tool(compose_tool)
    }
}

impl Director {
    pub fn new(deepseek_api_key: impl Into<String>) -> Self {
        Self {
            sources: Self::get_all_sources(deepseek_api_key),
            tts: None,
            subtitle: None,
            video_editor: None,
            voice_edit_tool: None,
            compose_tool: None,
        }
    }

    fn get_all_sources(deepseek_api_key: impl Into<String>) -> Vec<NewsSource> {
        let mut sources = vec![];

        sources.push(NewsSource {
            name: crate::news::pengpai_news::SOURCE_NAME.to_owned(),
            crawler: Box::new(crate::news::pengpai_news::PengPaiNews::new()),
            extractor: Box::new(
                crate::news::pengpai_news::PengPaiNewsMaterialExtractor::from_deepseek(
                    deepseek_api_key.into(),
                ),
            ),
        });

        sources
    }

    pub fn with_tts(self, tts: impl TTSService + Sync + Send + 'static) -> Self {
        Self {
            tts: Some(Box::new(tts)),
            ..self
        }
    }

    pub fn with_subtitle(self, subtitle: impl Subtitle + Sync + Send + 'static) -> Self {
        Self {
            subtitle: Some(Box::new(subtitle)),
            ..self
        }
    }

    pub fn with_voice_edit_tool(self, tool: impl VoiceEditTool + Sync + Send + 'static) -> Self {
        Self {
            voice_edit_tool: Some(Box::new(tool)),
            ..self
        }
    }

    pub fn with_compose_tool(self, tool: impl ComposeTool + Sync + Send + 'static) -> Self {
        Self {
            compose_tool: Some(Box::new(tool)),
            ..self
        }
    }

    pub fn with_video_editor(self, video_editor: impl VideoEditor + Sync + Send + 'static) -> Self {
        Self {
            video_editor: Some(Box::new(video_editor)),
            ..self
        }
    }

    pub async fn get_hot_news_list(&self, source_name: &SourceName) -> Vec<NewsTitle> {
        let source = self.sources.iter().find(|s| s.name == *source_name);
        if source.is_none() {
            return vec![];
        }

        let source = source.unwrap();

        source.crawler.get_hot_news_list().await
    }

    pub async fn shot_single(&self, news_title: NewsTitle) -> DirectorResult<NewsShortVideo> {
        // let temp_dir = self.get_temp_dir().await?;

        let source = self
            .sources
            .iter()
            .find(|s| s.name == *news_title.source)
            .ok_or(DirectorError::Source(format!(
                "Failed to find source: {}",
                news_title.source
            )))?;

        let material = source.extractor.get_material(&news_title).await?;

        let dubbing_path = if self.tts.is_some() {
            Some(self.gen_dubbing(&material).await?)
        } else {
            None
        };

        let subtitle_path = if let Some(ref subtitle_handler) = self.subtitle
            && let Some(ref dubbing_subtitle) = dubbing_path
        {
            Some(
                subtitle_handler
                    .write_subtitle(&dubbing_subtitle.tts_files)
                    .await?,
            )
        } else {
            None
        };

        let video_path = {
            let dur = if let Some(ref dubbing) = dubbing_path {
                Some(get_wav_len(&dubbing.dubbing_path).await?)
            } else {
                None
            };

            if self.video_editor.is_some() {
                Some(self.gen_video(&material, dur).await?)
            } else {
                None
            }
        };

        let video_path =
            video_path.ok_or(DirectorError::VideoEditor(VideoEditorError::Duration))?;

        let final_path = self
            .compose_all(
                video_path,
                dubbing_path.unwrap().dubbing_path,
                subtitle_path.unwrap(),
            )
            .await?;

        Ok(NewsShortVideo {
            title: material.title.clone(),
            path: final_path,
        })

    }

    async fn gen_dubbing(&self, material: &NewsMaterial) -> DirectorResult<DubbingSubtitle> {
        let mut tts_files = self
            .tts
            .as_ref()
            .expect("Has no TTS setted")
            .tts(&material.summary)
            .await?;

        // carton tts files
        for tts_file in tts_files.iter_mut() {
            let mut new_file_path = tts_file.path.clone();

            if let Some(tool) = &self.voice_edit_tool {
                new_file_path.set_file_name(format!(
                    "{}-cartoned.{}",
                    new_file_path.file_stem().unwrap().to_str().unwrap(),
                    new_file_path.extension().unwrap().to_str().unwrap()
                ));
                tool.cartoned_voice(&tts_file.path, &new_file_path).await?;
                _ = fs::remove_file(&tts_file.path).await;
            }

            tts_file.duration = get_wav_len(&new_file_path).await?;
            tts_file.path = new_file_path;
        }

        // compose up
        let compose_path = self.compose_audio(&tts_files).await?;

        let subtitles: Vec<SingleSubtitle> = tts_files
            .into_iter()
            .map(|tts| {
                _ = std::fs::remove_file(&tts.path);
                return SingleSubtitle {
                    text: tts.text.clone(),
                    duration: tts.duration,
                };
            })
            .collect();

        Ok(DubbingSubtitle {
            dubbing_path: compose_path,
            tts_files: subtitles,
        })
    }

    async fn compose_audio(&self, tts_files: &Vec<TTSFile>) -> DirectorResult<PathBuf> {
        if tts_files.is_empty() {
            return Err(DirectorError::TTS(TTSError::NoSet));
        }

        let spec = {
            let first = tts_files.get(0).unwrap();
            let reader = WavReader::open(&first.path)?;
            reader.spec()
        };

        let silence = {
            let silence_duration = 0.3;
            let num_samples = (spec.sample_rate as f32 * silence_duration) as usize;
            vec![0; num_samples * spec.channels as usize]
        };

        let mut compose_wav = vec![];

        for tts_file in tts_files {
            let mut reader = WavReader::open(&tts_file.path).unwrap();
            let samples: Vec<i32> = reader.samples::<i16>().map(|s| s.unwrap() as i32).collect();

            compose_wav.extend(samples.clone());
            compose_wav.extend(silence.clone());
        }

        let final_wav = self
            .get_temp_dir()
            .await?
            .join(format!("{}-final.wav", nanoid!()));

        let mut writer = hound::WavWriter::create(&final_wav, spec)?;

        for sample in compose_wav {
            writer.write_sample(sample).unwrap();
        }

        Ok(final_wav)
    }

    async fn compose_all(
        &self,
        video: PathBuf,
        dubbing: PathBuf,
        subtitle: PathBuf,
    ) -> DirectorResult<PathBuf> {
        let tool = self
            .compose_tool
            .as_ref()
            .expect("compose tool has not set");

        let output_path = self
            .get_temp_dir()
            .await?
            .join(format!("{}-final.mp4", nanoid::nanoid!()));

        tool.compose_all(&video, &dubbing, &subtitle, &output_path)
            .await?;

        _ = fs::remove_file(video).await;
        _ = fs::remove_file(dubbing).await;
        _ = fs::remove_file(subtitle).await;

        Ok(output_path)
    }

    async fn gen_video(
        &self,
        material: &NewsMaterial,
        dur: Option<Duration>,
    ) -> DirectorResult<PathBuf> {
        if self.video_editor.is_none() {
            return Err(DirectorError::VideoEditor(VideoEditorError::NoSet));
        }

        let time = if let Some(time) = dur {
            time
        } else {
            Duration::from_secs((material.pics.len() * 2) as u64)
        };

        let path = self
            .video_editor
            .as_ref()
            .unwrap()
            .do_edit(material, time)
            .await?;

        Ok(path)
    }

    async fn get_temp_dir(&self) -> DirectorResult<PathBuf> {
        let temp_dir = PathBuf::from(".").join("temp").join(nanoid!(10));

        if !temp_dir.exists() {
            fs::create_dir_all(&temp_dir).await?;
        }

        Ok(temp_dir)
    }
}
struct DubbingSubtitle {
    dubbing_path: PathBuf,
    tts_files: Vec<SingleSubtitle>,
}

#[cfg(test)]
mod test {

    use crate::{
        subtitle::{SingleSubtitle, SubtitleResult},
        tts::TTSFile,
        video::VideoEditorResult,
    };

    use super::*;

    #[test]
    fn new_director() {
        let director = Director::new("Fake DeepSeek API Key");

        assert_eq!(1, director.sources.len());
        assert!(director.tts.is_none());
        assert!(director.subtitle.is_none());
        assert!(director.video_editor.is_none());
    }

    #[test]
    fn new_director_with_compoments() {
        struct FakeTTS;
        #[async_trait::async_trait]
        impl TTSService for FakeTTS {
            async fn tts(&self, _: &Vec<String>) -> Result<Vec<TTSFile>, TTSError> {
                unimplemented!();
            }
        }

        let director = Director::new("Fake DeepSeek API Key").with_tts(FakeTTS);
        assert!(director.tts.is_some());

        struct FakeSubtitle;
        #[async_trait::async_trait]
        impl Subtitle for FakeSubtitle {
            async fn write_subtitle(&self, _: &Vec<SingleSubtitle>) -> SubtitleResult {
                unimplemented!();
            }
        }
        let director = Director::new("Fake DeepSeek API Key").with_subtitle(FakeSubtitle);
        assert!(director.subtitle.is_some());

        struct FakeVideoEditor;
        #[async_trait::async_trait]
        impl VideoEditor for FakeVideoEditor {
            async fn do_edit(&self, _: &NewsMaterial, _: Duration) -> VideoEditorResult<PathBuf> {
                unimplemented!();
            }
        }
        let director = Director::new("Fake DeepSeek API Key").with_video_editor(FakeVideoEditor);
        assert!(director.video_editor.is_some());
    }
}
