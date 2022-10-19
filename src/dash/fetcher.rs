use super::parser::PCCDashParser;
use anyhow::{Context, Result};
use futures::TryFutureExt;
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

async fn fetch_mpd(mpd_url: &str, http_client: &HttpClient) -> Result<String> {
    let resp = http_client.get(mpd_url).send().await?;
    let content = resp.text().await?;

    Ok(content)
}

impl Fetcher {
    pub async fn new<P: Into<PathBuf>>(mpd_url: &str, download_dir: P) -> Fetcher {
        let client = reqwest::Client::builder()
            .timeout(Duration::new(10, 0))
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
    // quality is representation id (0 is highest quality)
    pub async fn download(&self, object_id: u8, quality: u8, frame: u64) -> Result<PathBuf> {
        let url = self.parser.get_url(object_id, quality, frame);
        let output_path = self
            .download_dir
            .join(generate_filename_from_url(url.as_str()));
        println!("Downloading {} to {}", url, output_path.display());
        let (content, file) = tokio::join!(
            self.http_client
                .get(&url)
                .send()
                .and_then(|resp| resp.bytes()),
            File::create(&output_path)
        );
        println!("Downloaded {}", url);
        tokio::io::copy(&mut content?.as_ref(), &mut file?).await?;
        Ok(output_path)
    }

    pub fn get_total_frames(&self) -> usize {
        self.parser.get_total_frames()
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
