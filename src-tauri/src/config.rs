// use std::collections::HashMap;

use config::{Config, File};

const DEFAULT_CONFIG_FILE: &str = "config";

pub struct GlobalConfig {
    config: Config,
    // cache: HashMap<String, String>,
}

impl GlobalConfig {
    pub fn new() -> Self {
        let config = Config::builder()
            .add_source(File::with_name(DEFAULT_CONFIG_FILE))
            .build()
            .unwrap();

        Self { 
            config,
            // cache: HashMap::new(),
        }
    }

    pub fn get_tts_url(&self) -> String {
        self.config.get_string("TTS_URL").unwrap()
    }

    pub fn get_ali_dashscope_api_key(&self) -> String {
        self.config.get_string("ALI_DASHSCOPE_API_KEY").unwrap()
    }

    pub fn get_deepseek_api_key(&self) -> String {
        self.config.get_string("OPENAI_KEY").unwrap()
    }
}
