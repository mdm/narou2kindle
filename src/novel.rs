use std::{error::Error, path::PathBuf, ffi::{OsStr, OsString}};

use async_process::Command;
use tempfile::{tempdir, TempDir};
use tokio::{fs::File, io::AsyncWriteExt};

pub struct RawNovel {
    ncode: String,
    toc: String,
    chapters: Vec<String>,
}

impl RawNovel {
    pub async fn get(
        ncode: &str,
        progress: &mut indicatif::ProgressBar,
    ) -> Result<RawNovel, Box<dyn Error>> {
        let http_client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0")
            .build()?;

        let toc = http_client
            .get(format!("https://ncode.syosetu.com/{ncode}/")) // TODO: extract BASE_URL constant/config option
            .send()
            .await?
            .text()
            .await?;

        // println!("{}", &toc);

        let chapter_links = {
            let chapter_link_selector =
                scraper::Selector::parse("div.index_box > dl.novel_sublist2 > dd.subtitle > a")?;
            let parsed_toc = scraper::Html::parse_document(&toc); // TODO: use spawn_blocking()

            parsed_toc
                .select(&chapter_link_selector)
                .filter_map(|chapter_link| {
                    chapter_link
                        .value()
                        .attr("href")
                        .map(|href| href.to_string())
                })
                .collect::<Vec<_>>()
        };

        progress.set_length(chapter_links.len() as u64);

        let mut chapters = Vec::new();
        for path in chapter_links {
            let chapter = http_client
                .get(format!("https://ncode.syosetu.com{path}")) // TODO: extract BASE_URL constant
                .send()
                .await?
                .text()
                .await?;

            chapters.push(chapter);
            progress.inc(1);
        }

        Ok(RawNovel {
            ncode: ncode.to_string(),
            toc,
            chapters,
        })
    }
}

pub struct MarkdownNovel {
    ncode: String,
    dir: TempDir,
    metadata_path: PathBuf,
    chapter_paths: Vec<PathBuf>,
}

impl MarkdownNovel {
    pub async fn from_raw(raw_novel: &RawNovel) -> Result<MarkdownNovel, Box<dyn Error>> {
        let ncode = raw_novel.ncode.clone();
        let dir = tempdir()?;
        let metadata_path = dir.path().join(format!("{}-metadata.yaml", ncode));
        let chapter_paths = Vec::new();

        dbg!(&dir);

        let mut markdown_novel = MarkdownNovel {
            ncode,
            dir,
            metadata_path,
            chapter_paths,
        };

        markdown_novel.format_frontmatter(&raw_novel.toc).await?;
        for chapter in &raw_novel.chapters {
            markdown_novel.format_chapter(&chapter).await?;
        }

        Ok(markdown_novel)
    }

    async fn format_frontmatter(&mut self, raw_toc: &str) -> Result<(), Box<dyn Error>> {
        let (author, title, preface) = {
            let parsed_toc = scraper::Html::parse_document(&raw_toc);

            let author_selector =
                scraper::Selector::parse("div#novel_color > div.novel_writername > a")?;
            let author = parsed_toc
                .select(&author_selector)
                .next()
                .map(|author| author.text().collect::<String>());

            let author = match author {
                Some(author) => format!("author: {author}\n"),
                None => "".to_string(),
            };

            let title_selector = scraper::Selector::parse("div#novel_color > p.novel_title")?;
            let title = parsed_toc
                .select(&title_selector)
                .next()
                .map(|title| title.text().collect::<String>());

            let title = match title {
                Some(title) => format!("title: {title}\n"),
                None => "".to_string(),
            };

            let preface_selector = scraper::Selector::parse("div#novel_color > div#novel_ex")?;
            let preface = parsed_toc
                .select(&preface_selector)
                .next()
                .map(|preface| preface.text().collect::<String>().replace("<br>", "\n"));

            (author, title, preface)
        };

        let mut metadata_file = File::create(&self.metadata_path).await?;
        metadata_file.write_all("---\n".as_bytes()).await?;
        metadata_file.write_all(title.as_bytes()).await?;
        metadata_file.write_all(author.as_bytes()).await?;
        metadata_file
            .write_all("lang: ja-JP\n".as_bytes())
            .await?;
        metadata_file.write_all("---\n".as_bytes()).await?;

        if let Some(preface) = preface {
            let preface_path = self.dir.path().join(format!("{}-{:04}.md", self.ncode, 0));
            let mut preface_file = File::create(&preface_path).await?;
            preface_file.write_all(preface.as_bytes()).await?;
            preface_file.write_all("\n".as_bytes()).await?;

            self.chapter_paths.push(preface_path);
        }

        Ok(())
    }

    async fn format_chapter(&mut self, raw_chapter: &str) -> Result<(), Box<dyn Error>> {
        let (heading, intro, content, outro) = {
            let parsed_chapter = scraper::Html::parse_document(&raw_chapter);

            let heading_selector = scraper::Selector::parse("div#novel_color > p.novel_subtitle")?;
            let heading = parsed_chapter
                .select(&heading_selector)
                .next()
                .map(|heading: scraper::ElementRef| heading.text().collect::<String>());

            let intro_selector = scraper::Selector::parse("div#novel_color > div#novel_p")?;
            let intro = parsed_chapter
                .select(&intro_selector)
                .next()
                .map(|intro| intro.text().collect::<String>().replace("<br>", "\n"));

            let content_selector = scraper::Selector::parse("div#novel_color > div#novel_honbun")?;
            let content = parsed_chapter
                .select(&content_selector)
                .next()
                .map(|content| content.text().collect::<String>().replace("<br>", "\n"));

            let outro_selector = scraper::Selector::parse("div#novel_color > div#novel_a")?;
            let outro = parsed_chapter
                .select(&outro_selector)
                .next()
                .map(|outro| outro.text().collect::<String>().replace("<br>", "\n"));

            (heading, intro, content, outro)
        };

        let chapter_path =
            self.dir
                .path()
                .join(format!("{}-{:04}.md", self.ncode, self.chapter_paths.len()));
        let mut chapter_file = File::create(&chapter_path).await?;

        if let Some(heading) = heading {
            chapter_file.write_all("# ".as_bytes()).await?;
            chapter_file.write_all(heading.as_bytes()).await?;
            chapter_file.write_all("\n\n".as_bytes()).await?;
        }

        if let Some(intro) = intro {
            chapter_file.write_all(intro.as_bytes()).await?;
            chapter_file.write_all("\n---\n".as_bytes()).await?;
        }

        if let Some(content) = content {
            chapter_file.write_all(content.as_bytes()).await?;
            chapter_file.write_all("\n".as_bytes()).await?;
        }

        if let Some(outro) = outro {
            chapter_file.write_all("---\n".as_bytes()).await?;
            chapter_file.write_all(outro.as_bytes()).await?;
            chapter_file.write_all("\n".as_bytes()).await?;
        }

        self.chapter_paths.push(chapter_path);

        Ok(())
    }
}

pub struct EpubNovel {
    ncode: String,
    dir: TempDir,
    output_path: PathBuf,
}

impl EpubNovel {
    pub async fn from_markdown(
        markdown_novel: &MarkdownNovel,
    ) -> Result<EpubNovel, Box<dyn Error>> {
        let ncode = markdown_novel.ncode.clone();
        let dir = tempdir()?;

        dbg!(&dir);

        let output_path = dir.path().join(format!("{ncode}.epub"));
        // let mut output_arg = OsString::new();
        // output_arg.push("-o");
        // output_arg.push(output_path.as_os_str());
        Command::new("pandoc")
            .arg("-o")
            .arg(output_path.as_os_str())
            .arg(markdown_novel.metadata_path.as_os_str())
            .args(markdown_novel.chapter_paths.iter().map(|path| path.as_os_str()))
            .output().await?;

        Ok(EpubNovel {
            ncode,
            dir,
            output_path,
        })
    }
}
