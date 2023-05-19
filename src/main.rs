use std::time::Duration;

use clap::Parser;
use tokio::time::sleep;

mod novel;

#[derive(Debug, Parser)]
struct Cli {
    ncode: Vec<String>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let multi_progress = indicatif::MultiProgress::new();

    let mut handles = Vec::new();
    for ncode in cli.ncode {
        let mut progress = multi_progress.add(indicatif::ProgressBar::new(1));
        let handle = tokio::spawn(async move {
            let raw_novel = novel::RawNovel::get(&ncode, &mut progress).await.unwrap();
            let markdown_novel = novel::MarkdownNovel::from_raw(&raw_novel).await.unwrap();
            let epub_novel = novel::EpubNovel::from_markdown(&markdown_novel).await.unwrap();
            sleep(Duration::from_secs(30)).await;
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }
}
