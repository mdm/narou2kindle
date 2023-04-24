pub struct RawNovel {
    pub toc: String,
    pub chapters: Vec<String>,
}

impl RawNovel {
    pub async fn get(ncode: &str, progress: &mut indicatif::ProgressBar) -> Result<RawNovel, Box<dyn std::error::Error>> {
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

        Ok(RawNovel { toc, chapters })
    }
}
