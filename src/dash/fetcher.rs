use crate::dash::parser::PCCDashParser;
use anyhow::{Context, Result};
use rayon::prelude::*;
use std::collections::HashMap;
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

    // parallelized download of all segments
    pub fn download_to(
        &self,
        out: &PathBuf,
        representation_id: Option<u8>,
    ) -> Result<Vec<PathBuf>> {
        let mpd = self.fetch_mpd()?;
        let filepaths = mpd
            .get(&representation_id.unwrap_or(0).to_string())
            .unwrap()
            .into_par_iter()
            .filter_map(|url: &String| -> Option<PathBuf> {
                if let Ok(dest_path) = self.http_client.get(url).send().and_then(|mut resp| {
                    let dest_path = out.join(generate_filename_from_url(url.as_str()));
                    let mut dest = File::create(&dest_path).unwrap();
                    resp.copy_to(&mut dest).unwrap();
                    Ok(dest_path)
                }) {
                    Some(dest_path)
                } else {
                    None
                }
            })
            .collect::<Vec<PathBuf>>();
        Ok(filepaths)
    }

    fn fetch_mpd(&self) -> Result<HashMap<String, Vec<String>>> {
        let resp = self.http_client.get(&self.mpd_url).send()?;
        let content = resp.text()?;

        if self.mpd_url.ends_with("txt") {
            self.parse_mpd_txt(content.as_str())
        } else {
            self.parse_mpd_xml(content.as_str())
        }
    }

    fn parse_mpd_xml(&self, content: &str) -> Result<HashMap<String, Vec<String>>> {
        let mut result: HashMap<String, Vec<String>> = HashMap::new();

        let parser = PCCDashParser::new(content);
        for period in parser.get_periods() {
            for ad in parser.get_adaptation_sets(&period) {
                for repr in parser.get_representations(&ad) {
                    let st = parser.get_segment_template(&repr);
                    let mut urls = parser.expand_segment_urls(&st)?;

                    result
                        .entry(repr.attribute("id").unwrap().to_owned())
                        .or_insert(Vec::new())
                        .append(&mut urls);
                }
            }
        }

        Ok(result)
    }

    fn parse_mpd_txt(&self, content: &str) -> Result<HashMap<String, Vec<String>>> {
        Ok(HashMap::from([(
            "1".to_owned(),
            content
                .split("\n")
                .filter(|x| *x != "")
                .map(|x| x.to_owned())
                .collect(),
        )]))
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
    pub fn test_parse_mpd_xml() {
        let fetcher = Fetcher::new("http://localhost:3000/mpd.txt");
        assert_eq!(
            fetcher
                .parse_mpd_xml(
                    r#"<?xml version='1.0'?>
        <MPD xmlns="urn:mpeg:dash:schema:mpd:2011" 
            profiles="urn:mpeg:dash:profile:full:2011">
            <BaseURL>https://www.example.com/</BaseURL> 
            <Period duration="PT120S">
                <AdaptationSet id="1" mimeType="application/mpegvpcc"
                    xRot="0" yRot="0" zRot="0" xOff="0" yOff="0"
                    zOff="0">
                    <Representation id="1" bandwidth="2400000"> 
                        <SegmentTemplate
                            media="loot/1/segment_$Number%04u$.bin"
                            duration="30" timescale="30" startNumber="1"/>
                    </Representation>
                    <Representation id="2" bandwidth="3620000">
                        <SegmentTemplate
                            media="loot/2/segment_$Number%04u$.bin"
                            duration="30" timescale="30" startNumber="1"/> 
                    </Representation>
                <!-- further representations -->
                </AdaptationSet>
                <AdaptationSet id="2" mimeType="application/mpegvpcc"
                    xRot="0" yRot="3.1416" zRot="0" xOff="2000" yOff="0"
                    zOff="0">
                    <Representation id="1" bandwidth="3500000"> <SegmentTemplate
                        media="redandblack/1/segment_$Number%04u$.bin"
                        duration="30" timescale="30" startNumber="1"/>
                    </Representation>
                    <!-- further representations -->
                </AdaptationSet>
                <!-- further adaptation sets -->
            </Period>
            <!-- further periods -->
        </MPD>
                "#
                )
                .unwrap(),
            HashMap::from([
                (
                    "1".to_owned(),
                    vec![
                        "https://www.example.com/loot/1/segment_$Number%04u$.bin".to_owned(),
                        "https://www.example.com/redandblack/1/segment_$Number%04u$.bin".to_owned()
                    ]
                ),
                (
                    "2".to_owned(),
                    vec!["https://www.example.com/loot/2/segment_$Number%04u$.bin".to_owned()]
                )
            ])
        )
    }
    #[test]
    pub fn test_download_to() {
        let fetcher = Fetcher::new("http://localhost:3000/mpd.txt");
        assert_eq!(
            fetcher
                .download_to(&PathBuf::from("test_files"), Some(2))
                .expect("couldn't download files")
                .len(),
            2,
        );
    }
}
