mod downloader;
mod formatter;
mod converter;
mod mailer;

#[tokio::main]
async fn main() {
    let raw_novel = downloader::RawNovel::get("n0775id").await.unwrap();
}
