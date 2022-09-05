use anyhow::{Context, Result};
use std::fs::File;
use std::path::PathBuf;
use std::time::Duration;

pub type HttpClient = reqwest::blocking::Client;

pub struct Fetcher {
    mpd_url: String,
    http_client: HttpClient,
}

impl Fetcher {
    pub fn new(mpd_url: &str) -> Fetcher {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::new(10, 0))
            .gzip(true)
            .build()
            .context("building reqwest HTTP client")
            .unwrap();

        Fetcher {
            mpd_url: String::from(mpd_url),
            http_client: client,
        }
    }

    pub fn download_to(self, out: &PathBuf) -> Result<()> {
        let mpd = self.fetch_mpd()?;
        for url in mpd.split("\n") {
            if url == "" {
                // HACK: ignore empty lines and break
                break;
            }
            let mut resp = self.http_client.get(url).send()?;
            let mut dest = File::create(out.join(generate_filename_from_url(url)))?;
            resp.copy_to(&mut dest)?;
        }
        Ok(())
    }

    fn fetch_mpd(&self) -> Result<String> {
        let resp = self.http_client.get(&self.mpd_url).send()?;
        let content = resp.text()?;
        Ok(content)
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

    #[test]
    pub fn test_download_to() {
        let fetcher = Fetcher::new("http://localhost:3000/mpd.txt");
        assert_eq!(
            fetcher
                .download_to(&PathBuf::from("test_files"))
                .expect("couldn't download files"),
            ()
        );
    }
}
