use super::parser::MPDParser;
use anyhow::{Context, Result};
use futures::future;
use log::debug;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs::File;

pub type HttpClient = reqwest::Client;

#[derive(Clone)]
pub struct Fetcher {
    http_client: HttpClient,
    mpd_parser: MPDParser,
    download_dir: PathBuf,
    pub stats: FetchStats,
}

#[derive(Clone, Debug)]
pub struct FetchStats {
    pub avg_bitrate: SimpleRunningAverage<5>,
}

#[derive(Clone, Debug)]
pub struct SimpleRunningAverage<const N: usize> {
    values: [usize; N],
    /// pointer to the next value to be overwritten
    next: usize,
    avg: usize,
    divide_by: usize,
}

impl<const N: usize> SimpleRunningAverage<N> {
    fn new() -> Self {
        SimpleRunningAverage {
            values: [0; N],
            next: 0,
            avg: 0,
            divide_by: 1,
        }
    }

    /// Adds a new datapoint to the running average, removing the oldest
    fn add(&mut self, value: usize) {
        self.avg = self.avg + (value - self.values[self.next as usize]) / self.divide_by;
        self.values[self.next as usize] = value;
        self.next = (self.next + 1) % N;
        self.divide_by = std::cmp::min(self.divide_by + 1, N);
    }

    /// Gets the running average
    fn get(&self) -> usize {
        self.avg
    }
}

#[derive(Debug)]
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
            mpd_parser: MPDParser::new(&mpd),
            download_dir: download_dir.into(),
            stats: FetchStats {
                avg_bitrate: SimpleRunningAverage::new(),
            },
        }
    }

    // object_id is adaptation set id
    pub async fn download(&mut self, object_id: u8, frame: u64) -> Result<FetchResult> {
        let mut paths = core::array::from_fn(|_| None);

        // quality is representation id (0 is lowest quality)
        let mut urls: [String; 6] = core::array::from_fn(|_| String::new());
        let mut bandwidths = [None; 6];

        for view_id in 0..6 {
            // TODO: fix hard code representation id
            let (url, bandwidth) = self.mpd_parser.get_info(object_id, 1, view_id as u8, frame);
            urls[view_id] = url;
            bandwidths[view_id] = bandwidth;
            let output_path = self
                .download_dir
                .join(generate_filename_from_url(urls[view_id].as_str()));
            paths[view_id] = Some(output_path);
        }
        let now = std::time::Instant::now();

        // If file exists, then there is no need to download again.
        let contents = future::join_all(urls.into_iter().map(|url| {
            let client = &self.http_client;
            let filename = generate_filename_from_url(url.as_str());
            let local_file_path = self.download_dir.join(filename);
            async move {
                let f = File::open(local_file_path).await;
                if let Ok(_) = f {
                    // File exists so we should skip downloading
                    Ok(None)
                } else {
                    let resp = client.get(url).send().await?;
                    match resp.error_for_status() {
                        Ok(resp) => Ok(resp.bytes().await.ok()),
                        Err(e) => Err(e),
                    }
                }
            }
        }))
        .await;

        let elapsed = now.elapsed();

        let total_bits = contents
            .iter()
            .filter(|c| c.is_ok())
            .map(|c| c.as_ref().unwrap())
            .filter(|c| c.is_some())
            .map(|c| c.as_ref().unwrap().len())
            .sum::<usize>()
            * 8;

        self.stats
            .avg_bitrate
            .add(total_bits / std::cmp::max(1, elapsed.as_millis()) as usize);
        debug!(
            "download time: {:?}, bits: {:}, avg_bitrate(latest): {:?}kbps",
            elapsed,
            total_bits,
            self.stats.avg_bitrate.get()
        );

        for (i, content) in contents.into_iter().enumerate() {
            if let Ok(Some(content)) = content {
                let mut file = File::create(&paths[i].clone().unwrap()).await?;
                tokio::io::copy(&mut content.as_ref(), &mut file).await?;
            } else if let Err(e) = content {
                eprintln!("Error downloading file: {}", e);
                return Err(e.into());
            }
        }
        Ok(FetchResult(paths))
    }

    pub fn total_frames(&self) -> usize {
        self.mpd_parser.total_frames()
    }

    pub fn segment_size(&self) -> u64 {
        self.mpd_parser.segment_size()
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
