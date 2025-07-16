pub mod config;
pub mod director;
pub mod mpeg;
pub mod news;
pub mod subtitle;
pub mod tts;
pub mod video;

use tauri::{AppHandle, Manager, State};
use tokio::sync::RwLock;

use crate::config::GlobalConfig;
use crate::director::Director;
use crate::director::source::SourceName;
use crate::news::NewsTitle;

#[tauri::command]
async fn get_hot_news_list(
    source: SourceName,
    app: AppHandle,
    config: State<'_, RwLock<GlobalConfig>>,
) -> Result<Vec<NewsTitle>, ()> {
    let config_g = config.read().await;
    let director = Director::default(
        config_g.get_tts_url(),
        config_g.get_ali_dashscope_api_key(),
        config_g.get_deepseek_api_key(),
        app,
    );

    let list = director.get_hot_news_list(&source).await;
    Ok(list)
}

#[tauri::command]
async fn gen_video(
    news_title: NewsTitle,
    app: AppHandle,
    config: State<'_, RwLock<GlobalConfig>>,
) -> Result<String, ()> {
    let config_g = config.read().await;
    let director = Director::default(
        config_g.get_tts_url(),
        config_g.get_ali_dashscope_api_key(),
        config_g.get_deepseek_api_key(),
        app,
    );

    let res = director.shot_single(news_title).await.unwrap();

    Ok(res.path.canonicalize().unwrap().display().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            app.manage(RwLock::new(crate::config::GlobalConfig::new()));
            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_hot_news_list, gen_video])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
