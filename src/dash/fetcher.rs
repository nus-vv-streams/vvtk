use super::parser::PCCDashParser;
use anyhow::{Context, Result};
use futures::pin_mut;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use std::time::Duration;

pub type HttpClient = reqwest::Client;

pub struct Fetcher {
    mpd_url: String,
    http_client: HttpClient,
    mpd: PCCDashParser,
}

// Assume all frames is shown in 30 fps
pub(super) struct Segment {
    template: String,
    start_frame: u64,
    duration_in_frames: u64, // in frames.
    representation_id: u8,
    adaptation_set_id: u8,
}

async fn fetch_mpd(mpd_url: &str, http_client: &HttpClient) -> Result<String> {
    let resp = http_client.get(mpd_url).send().await?;
    let content = resp.text().await?;

    Ok(content)
}

impl Fetcher {
    pub async fn new(mpd_url: &str) -> Fetcher {
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
            mpd_url: String::from(mpd_url),
            http_client: client,
            mpd: PCCDashParser::new(&mpd),
        }
    }

    // object_id is adaptation set id
    // quality is representation id (0 is highest quality)
    pub async fn download(&self, object_id: u8, quality: u8, frame: u64) -> Result<Vec<PathBuf>> {
        let dur = frame / 30;
        // let filepaths = mpd
        //     .get(&representation_id.unwrap_or(0).to_string())
        //     .unwrap()
        //     .into_par_iter()
        //     .filter_map(|url: &String| -> Option<PathBuf> {
        //         if let Ok(dest_path) =
        //             self.http_client.get(url).send().await.and_then(|mut resp| {
        //                 let dest_path = self
        //                     .download_dir
        //                     .join(generate_filename_from_url(url.as_str()));
        //                 let mut dest = File::create(&dest_path).unwrap();
        //                 resp.await.copy_to(&mut dest).unwrap();
        //                 Ok(dest_path)
        //             })
        //         {
        //             Some(dest_path)
        //         } else {
        //             None
        //         }
        //     })
        //     .collect::<Vec<PathBuf>>();
        // Ok(filepaths)
        Ok(vec![])
    }

    // async fn fetch_mpd(mpd_url: &str, http_client: HttpClient) -> Result<String> {
    //     let resp = http_client.get(mpd_url).send().await?;
    //     let content = resp.text().await?;

    //     // if self.mpd_url.ends_with("txt") {
    //     //     self.parse_mpd_txt(content.as_str())
    //     // } else {
    //     //     self.parse_mpd_xml(content.as_str())
    //     // }
    //     Ok(content)
    // }

    // assumes that
    fn parse_mpd_xml(&self, content: &str) -> Result<Vec<Vec<Segment>>> {
        let mut result: Vec<Vec<Segment>> = vec![];

        // let parser = PCCDashParser::new(content);
        // for period in parser.get_periods() {
        //     for ad in parser.get_adaptation_sets(&period) {
        //         let mut segments: Vec<Segment> = vec![];
        //         for repr in parser.get_representations(&ad) {
        //             let st = parser.get_segment_template(&repr);
        //             segments.push(parser.get_segment(&st)?);
        //         }
        //         result.push(segments)
        //     }
        // }

        Ok(result)
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

    // #[tokio::test]
    // pub async fn test_parse_mpd_xml() {
    //     let fetcher = Fetcher::new("http://localhost:3000/mpd.txt").await;
    //     assert_eq!(
    //         fetcher
    //             .parse_mpd_xml(
    //                 r#"<?xml version='1.0'?>
    //     <MPD xmlns="urn:mpeg:dash:schema:mpd:2011"
    //         profiles="urn:mpeg:dash:profile:full:2011">
    //         <BaseURL>https://www.example.com/</BaseURL>
    //         <Period duration="PT120S">
    //             <AdaptationSet id="1" mimeType="application/mpegvpcc"
    //                 xRot="0" yRot="0" zRot="0" xOff="0" yOff="0"
    //                 zOff="0">
    //                 <Representation id="1" bandwidth="2400000">
    //                     <SegmentTemplate
    //                         media="loot/1/segment_$Number%04u$.bin"
    //                         duration="30" timescale="30" startNumber="1"/>
    //                 </Representation>
    //                 <Representation id="2" bandwidth="3620000">
    //                     <SegmentTemplate
    //                         media="loot/2/segment_$Number%04u$.bin"
    //                         duration="30" timescale="30" startNumber="1"/>
    //                 </Representation>
    //             <!-- further representations -->
    //             </AdaptationSet>
    //             <AdaptationSet id="2" mimeType="application/mpegvpcc"
    //                 xRot="0" yRot="3.1416" zRot="0" xOff="2000" yOff="0"
    //                 zOff="0">
    //                 <Representation id="1" bandwidth="3500000"> <SegmentTemplate
    //                     media="redandblack/1/segment_$Number%04u$.bin"
    //                     duration="30" timescale="30" startNumber="1"/>
    //                 </Representation>
    //                 <!-- further representations -->
    //             </AdaptationSet>
    //             <!-- further adaptation sets -->
    //         </Period>
    //         <!-- further periods -->
    //     </MPD>
    //             "#
    //             )
    //             .await
    //             .unwrap(),
    //         HashMap::from([
    //             (
    //                 "1".to_owned(),
    //                 vec![
    //                     "https://www.example.com/loot/1/segment_$Number%04u$.bin".to_owned(),
    //                     "https://www.example.com/redandblack/1/segment_$Number%04u$.bin".to_owned()
    //                 ]
    //             ),
    //             (
    //                 "2".to_owned(),
    //                 vec!["https://www.example.com/loot/2/segment_$Number%04u$.bin".to_owned()]
    //             )
    //         ])
    //     )
    // }
    // #[tokio::test]
    // pub async fn test_download_to() {
    //     let fetcher = Fetcher::new("http://localhost:3000/mpd.txt");
    //     assert_eq!(
    //         fetcher
    //             .download_to(&PathBuf::from("test_files"), Some(2))
    //             .await
    //             .expect("couldn't download files")
    //             .len(),
    //         2,
    //     );
    // }
}
