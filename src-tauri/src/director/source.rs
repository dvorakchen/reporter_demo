use crate::news::{MaterialExtractor, NewsCrawler};

pub type SourceName = String;

/// news source, where the news from and how handle it
pub struct NewsSource {
    pub name: SourceName,
    pub crawler: Box<dyn NewsCrawler + Sync + Send + 'static>,
    pub extractor: Box<dyn MaterialExtractor + Sync + Send + 'static>,
}

impl NewsSource {
    pub fn new(
        name: String,
        crawler: Box<dyn NewsCrawler + Sync + Send + 'static>,
        extractor: Box<dyn MaterialExtractor + Sync + Send + 'static>,
    ) -> Self {
        Self {
            name,
            crawler,
            extractor,
        }
    }
}
