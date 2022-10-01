use anyhow::{bail, Result};
use regex::Regex;
use roxmltree::{Document, Node};
use std::time::Duration;

pub struct PCCDashParser<'a> {
    doc: Document<'a>,
}

impl<'a> PCCDashParser<'_> {
    pub fn new(s: &str) -> PCCDashParser {
        let doc = Document::parse(s).unwrap();
        PCCDashParser { doc: doc }
    }

    pub fn get_periods(&self) -> Vec<Node> {
        self.doc
            .descendants()
            .filter(|n| n.has_tag_name("Period"))
            .collect()
    }

    pub fn get_base_url(&self) -> String {
        self.doc
            .descendants()
            .find(|n| n.has_tag_name("BaseURL"))
            .expect("Provide a BaseURL")
            .text()
            .expect("BaseURL cannot be empty")
            .to_owned()
    }

    pub fn get_adaptation_sets(&'a self, period: &Node<'a, 'a>) -> Vec<Node<'a, 'a>> {
        period
            .children()
            .filter(|n| n.has_tag_name("AdaptationSet"))
            .collect()
    }

    pub fn get_representations(&'a self, adaptation_set: &Node<'a, 'a>) -> Vec<Node<'a, 'a>> {
        adaptation_set
            .children()
            .filter(|n| n.has_tag_name("Representation"))
            .collect()
    }

    pub fn get_segment_template(&'a self, representation: &Node<'a, 'a>) -> Node<'a, 'a> {
        representation
            .children()
            .find(|n| n.has_tag_name("SegmentTemplate"))
            .expect("no segment templates found!")
    }

    pub fn get_duration_in_seconds(&'a self, period: &Node<'a, 'a>) -> Result<Duration> {
        let duration = period
            .attribute("duration")
            .expect("Provide a duration for the period");

        parse_xs_duration(duration)
    }

    pub fn expand_segment_urls(&'a self, segment_template: &Node<'a, 'a>) -> Result<Vec<String>> {
        let period = segment_template
            .ancestors()
            .find(|n| n.has_tag_name("Period"))
            .unwrap();
        let representation_id = segment_template
            .ancestors()
            .find(|n| n.has_tag_name("Representation"))
            .unwrap()
            .attribute("id")
            .unwrap();
        let period_dur = self.get_duration_in_seconds(&period).unwrap();
        let segment_dur = segment_template
            .attribute("duration")
            .expect("Provide a duration for the segment template")
            .parse::<u64>()?;
        let timescale = segment_template
            .attribute("timescale")
            .unwrap_or("1")
            .parse::<u64>()?;

        let number_of_frames = period_dur.as_secs() * timescale / segment_dur;
        let media = self.get_base_url()
            + Regex::new(r"\$RepresentationID\$")
                .unwrap()
                .replace(
                    segment_template
                        .attribute("media")
                        .expect("Provide a media attribute for the segment template"),
                    representation_id,
                )
                .as_ref();
        let start_number = segment_template
            .attribute("startNumber")
            .unwrap_or("1")
            .parse::<u64>()?;

        let number_re = Regex::new(r"\$Number\$").unwrap();
        Ok((0..number_of_frames)
            .map(|i| {
                number_re
                    .replace(&media, &format!("{}", start_number + i))
                    .to_string()
            })
            .collect())
    }
}

// Modified from https://github.com/emarsden/dash-mpd-rs
//
// Parse an XML duration string, as per https://www.w3.org/TR/xmlschema-2/#duration
//
// The lexical representation for duration is the ISO 8601 extended format PnYn MnDTnH nMnS, where
// nY represents the number of years, nM the number of months, nD the number of days, 'T' is the
// date/time separator, nH the number of hours, nM the number of minutes and nS the number of
// seconds. The number of seconds can include decimal digits to arbitrary precision.
//
// Examples: "PT0H0M30.030S", "PT1.2S", PT1004199059S, PT130S
// P2Y6M5DT12H35M30S	=> 2 years, 6 months, 5 days, 12 hours, 35 minutes, 30 seconds
// P1DT2H => 1 day, 2 hours
// P0Y20M0D => 20 months (0 is permitted as a number, but is not required)
// PT1M30.5S => 1 minute, 30.5 seconds
//
// Limitations: we can't represent negative durations (leading "-" character) due to the choice of a
// std::time::Duration. We only accept fractional parts of seconds, and reject for example "P0.5Y" and "PT2.3H".
fn parse_xs_duration(s: &str) -> Result<Duration> {
    let re = Regex::new(concat!(
        r"^(?P<sign>[+-])?P",
        r"(?:(?P<years>\d+)Y)?",
        r"(?:(?P<months>\d+)M)?",
        r"(?:(?P<weeks>\d+)W)?",
        r"(?:(?P<days>\d+)D)?",
        r"(?:(?P<hastime>T)", // time part must begin with a T
        r"(?:(?P<hours>\d+)H)?",
        r"(?:(?P<minutes>\d+)M)?",
        r"(?:(?P<seconds>\d+)(?:(?P<nanoseconds>[.,]\d+)?)S)?",
        r")?"
    ))
    .unwrap();
    match re.captures(s) {
        Some(m) => {
            if m.name("hastime").is_none()
                && m.name("years").is_none()
                && m.name("months").is_none()
                && m.name("weeks").is_none()
                && m.name("days").is_none()
            {
                bail!("empty");
            }
            let mut secs: u64 = 0;
            let mut nsecs: u32 = 0;
            if let Some(s) = m.name("nanoseconds") {
                let mut s = &s.as_str()[1..]; // drop initial "."
                if s.len() > 9 {
                    s = &s[..9];
                }
                let padded = format!("{:0<9}", s);
                nsecs = padded.parse::<u32>().unwrap();
            }
            if let Some(s) = m.name("seconds") {
                let seconds = s.as_str().parse::<u64>().unwrap();
                secs += seconds;
            }
            if let Some(s) = m.name("minutes") {
                let minutes = s.as_str().parse::<u64>().unwrap();
                secs += minutes * 60;
            }
            if let Some(s) = m.name("hours") {
                let hours = s.as_str().parse::<u64>().unwrap();
                secs += hours as u64 * 60 * 60;
            }
            if let Some(s) = m.name("days") {
                let days = s.as_str().parse::<u64>().unwrap();
                secs += days as u64 * 60 * 60 * 24;
            }
            if let Some(s) = m.name("weeks") {
                let weeks = s.as_str().parse::<u64>().unwrap();
                secs += weeks as u64 * 60 * 60 * 24 * 7;
            }
            if let Some(s) = m.name("months") {
                let months = s.as_str().parse::<u64>().unwrap();
                secs += months as u64 * 60 * 60 * 24 * 30;
            }
            if let Some(s) = m.name("years") {
                let years = s.as_str().parse::<u64>().unwrap();
                secs += years as u64 * 60 * 60 * 24 * 365;
            }
            if let Some(s) = m.name("sign") {
                if s.as_str() == "-" {
                    bail!("can't represent negative durations");
                }
            }
            Ok(Duration::new(secs, nsecs))
        }
        None => bail!("couldn't parse XS duration"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_run() {
        let p = PCCDashParser::new(
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
        "#,
        );
        assert_eq!(p.get_base_url(), "https://www.example.com/");

        let periods = p.get_periods();
        assert_eq!(periods.len(), 1);

        let first_period = periods.get(0).unwrap();
        assert_eq!(first_period.attribute("duration"), Some("PT120S"));
        let ads = p.get_adaptation_sets(first_period);
        assert_eq!(ads.len(), 2);

        let first_ad = ads.get(1).unwrap();
        assert_eq!(first_ad.attribute("id"), Some("2"));
        let reprs = p.get_representations(first_ad);
        assert_eq!(reprs.len(), 1);
    }

    #[test]
    pub fn test_run2() {
        let p = PCCDashParser::new(
            r#"<?xml version="1.0" encoding="UTF-8"?>
            <MPD format="pointcloud/pcd" type="static">
                <BaseURL>http://localhost:3000/</BaseURL>
                <Period id="1" duration="PT10S">
                    <AdaptationSet id="0">
                        <Representation id="0" bandwidth="13631488">
                            <SegmentTemplate media="longdress/$RepresentationID$/longdress_vox10_$Number$" duration="1" timescale="30" startNumber="1051"></SegmentTemplate>
                        </Representation>
                        <Representation id="1" bandwidth="1536000">
                            <SegmentTemplate media="longdress/$RepresentationID$/longdress_vox10_$Number$" duration="1" timescale="30" startNumber="1051"></SegmentTemplate>
                        </Representation>
                        <Representation id="2" bandwidth="204800">
                            <SegmentTemplate media="longdress/$RepresentationID$/longdress_vox10_$Number$" duration="1" timescale="30" startNumber="1051"></SegmentTemplate>
                        </Representation>
                    </AdaptationSet>
                </Period>
            </MPD>"#,
        );

        let periods = p.get_periods();
        let first_period = periods.get(0).unwrap();
        assert_eq!(
            p.get_duration_in_seconds(first_period).unwrap(),
            Duration::new(10, 0)
        );
        let ads = p.get_adaptation_sets(first_period);
        assert_eq!(ads.len(), 1);

        let first_ad = ads.get(0).unwrap();
        let reprs = p.get_representations(first_ad);
        assert_eq!(reprs.len(), 3);
        let segment_template = p.get_segment_template(reprs.get(2).unwrap());
        let expanded = p.expand_segment_urls(&segment_template).unwrap();
        assert_eq!(expanded.len(), 300);
        assert_eq!(
            expanded.get(0).unwrap().to_owned(),
            p.get_base_url() + "longdress/2/longdress_vox10_1051"
        );
    }
}
