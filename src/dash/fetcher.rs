use super::parser::PCCDashParser;
use anyhow::{Context, Result};
use futures::future;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs::File;

pub type HttpClient = reqwest::Client;

#[derive(Clone)]
pub struct Fetcher {
    http_client: HttpClient,
    parser: PCCDashParser,
    download_dir: PathBuf,
}

pub struct FetchResult(pub [Option<PathBuf>; 6]);

async fn fetch_mpd(mpd_url: &str, http_client: &HttpClient) -> Result<String> {
    let resp = http_client.get(mpd_url).send().await?;
    let content = resp.text().await?;

    Ok(content)
}

impl Fetcher {
    pub async fn new<P: Into<PathBuf>>(mpd_url: &str, download_dir: P) -> Fetcher {
        let client = reqwest::Client::builder()
            .timeout(Duration::new(30, 0))
            .gzip(true)
            .build()
            .context("building reqwest HTTP client")
            .unwrap();

        let mpd = fetch_mpd(mpd_url, &client)
            .await
            .expect("failed to fetch mpd");

        Fetcher {
            http_client: client,
            parser: PCCDashParser::new(&mpd),
            download_dir: download_dir.into(),
        }
    }

    // object_id is adaptation set id
    pub async fn download(&self, object_id: u8, frame: u64) -> Result<FetchResult> {
        let mut paths = core::array::from_fn(|_| None);

        // quality is representation id (0 is highest quality)
        let mut urls: [String; 6] = core::array::from_fn(|_| String::new());

        for i in 0..6 {
            // TODO: fix hard code representation id
            urls[i] = self.parser.get_url(object_id, 1, i as u8, frame);
            let output_path = self
                .download_dir
                .join(generate_filename_from_url(urls[i].as_str()));
            paths[i] = Some(output_path);
        }
        // TODO: add check if file exists.. no need to download again..
        // let now = std::time::Instant::now();
        // trace!(
        //     "[Fetcher] ({:?}) Downloading {} to {}",
        //     now,
        //     url,
        //     output_path.display()
        // );
        let contents = future::join_all(urls.into_iter().map(|url| {
            let client = &self.http_client;
            async move {
                dbg!(&url);
                let resp = client.get(url).send().await?;
                resp.bytes().await
            }
        }))
        .await;
        // let elapsed = now.elapsed();
        // let now = std::time::Instant::now();
        // info!(
        //     "[Fetcher] ({:?}) downloaded frame {} in {}.{:06}us",
        //     now,
        //     frame,
        //     elapsed.as_secs(),
        //     elapsed.subsec_micros(),
        // );
        for (i, content) in contents.into_iter().enumerate() {
            let mut file = File::create(&paths[i].clone().unwrap()).await?;
            tokio::io::copy(&mut content?.as_ref(), &mut file).await?;
        }
        // tokio::io::copy(&mut content?.as_ref(), &mut file?).await?;
        // let elapsed = now.elapsed();
        // debug!(
        //     "[Fetcher] ({:?}) write frame {} in {}.{:06}us",
        //     now,
        //     frame,
        //     elapsed.as_secs(),
        //     elapsed.subsec_micros(),
        // );
        Ok(FetchResult(paths))
    }

    pub fn total_frames(&self) -> usize {
        self.parser.total_frames()
    }

    pub fn segment_size(&self) -> u64 {
        self.parser.segment_size()
    }
}

fn generate_filename_from_url(url: &str) -> &str {
    url.rsplit_terminator("/").nth(0).unwrap()
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    pub fn test_generate_filename_from_url() {
        assert_eq!(
            generate_filename_from_url("https://www.example.com/papaya.txt"),
            "papaya.txt"
        );
        assert_eq!(
            generate_filename_from_url("https://www.example.com/p/a/paya.ply"),
            "paya.ply"
        );
    }
}
