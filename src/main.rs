use clap::Parser;
use tokio::join;

mod converter;
mod downloader;
mod formatter;
mod mailer;

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
            let raw_novel = downloader::RawNovel::get(&ncode, &mut progress).await.unwrap();
            // println!("{}: {}", &ncode, raw_novel.chapters.len());
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }
}
