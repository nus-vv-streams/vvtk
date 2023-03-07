//! Heavily simplified implementation of the MPD parser.
//! Adapted from https://github.com/emarsden/dash-mpd-rs/src/lib.rs

#![allow(non_snake_case)]

use anyhow::{bail, Result};
use regex::Regex;
use serde::de;
use serde::{Deserialize, Serialize, Serializer};
use serde_with::skip_serializing_none;
use std::collections::HashMap;
use std::time::Duration;

const FPS: u64 = 30;

#[derive(Clone)]
pub(crate) struct MPDParser {
    mpd: MPD,
    /// contains the first frame offsets for all `Period` in the MPD and the total number of frames.
    period_markers: Vec<u64>,
}

impl MPDParser {
    pub fn new(xml: &str) -> MPDParser {
        let mpd = MPD::from_xml(xml).unwrap();

        let mut framestamps: Vec<u64> = vec![];
        let mut curr_frame = 0;
        framestamps.push(curr_frame);
        for period in mpd.periods.iter() {
            if let Some(duration) = period.duration {
                curr_frame += (duration.as_secs_f64() * FPS as f64) as u64;
                framestamps.push(curr_frame);
            }
        }

        MPDParser {
            mpd,
            period_markers: framestamps,
        }
    }

    /// gets MPD's top-level BaseURL
    pub fn get_base_url(&self) -> String {
        let url = self
            .mpd
            .base_urls
            .as_ref()
            .unwrap()
            .get(0)
            .expect("no base url found")
            .base
            .clone();
        if url.ends_with('/') {
            url
        } else {
            url + "/"
        }
    }

    /// Get the number of frames in the whole MPD.
    pub fn total_frames(&self) -> usize {
        *self.period_markers.last().unwrap() as usize
    }

    /// Get the segment template's duration. To get the time in seconds, need to divide by segment template's timescale.
    pub fn segment_duration(&self) -> u64 {
        self.mpd.periods[0].adaptations.as_ref().unwrap()[0]
            .representations
            .as_ref()
            .unwrap()[0]
            .segment_template
            .as_ref()
            .unwrap()
            .duration
            .unwrap()
    }

    // From https://dashif.org/docs/DASH-IF-IOP-v4.3.pdf:
    // "For the avoidance of doubt, only %0[width]d is permitted and no other identifiers. The reason
    // is that such a string replacement can be easily implemented without requiring a specific library."
    //
    // Instead of pulling in C printf() or a reimplementation such as the printf_compat crate, we reimplement
    // this functionality directly.
    //
    // Example template: "$RepresentationID$/$Number%06d$.m4s"
    fn resolve_url_template(&self, template: &str, params: &HashMap<&str, String>) -> String {
        let mut result = template.to_string();
        for k in ["RepresentationID", "Number", "Time", "Bandwidth"] {
            // first check for simple case eg $Number$
            let ident = format!("${k}$");
            if result.contains(&ident) {
                if let Some(value) = params.get(k as &str) {
                    result = result.replace(&ident, value);
                }
            }
            // now check for complex case eg $Number%06d$
            let re = format!("\\${k}%0([\\d])d\\$");
            let ident_re = Regex::new(&re).unwrap();
            if let Some(cap) = ident_re.captures(&result) {
                if let Some(value) = params.get(k as &str) {
                    let width: usize = cap[1].parse::<usize>().unwrap();
                    let count = format!("{value:0>width$}");
                    let m = ident_re.find(&result).unwrap();
                    result = result[..m.start()].to_owned() + &count + &result[m.end()..];
                }
            }
        }
        result
    }

    /// gets the URL and the bandwidth information for the requested segment.
    ///
    /// # Arguments
    ///
    /// * `object_id` - Object ID of the requested segment
    /// * `representation_id` - quality of the requested segment
    /// * `frame offset` - Frame offset as calculated from the beginning of the video / MPD
    /// * `view_id` - View ID of the requested segment. If `None`, the parser assumes the pointclouds are not segmented into different planes, and will return the info for the first matching segment.
    pub fn get_info(
        &self,
        object_id: u8,
        representation_id: u8,
        frame_offset: u64,
        view_id: Option<u8>,
    ) -> (String, Option<u64>) {
        let period_idx =
            match self.period_markers[..].binary_search_by(|probe| probe.cmp(&frame_offset)) {
                Ok(idx) => idx,
                Err(idx) => idx - 1,
            };

        let base_url = self.get_base_url();
        let period = self.mpd.periods.get(period_idx).unwrap();
        let adaptation_set = period
            .adaptations
            .as_ref()
            .unwrap()
            .iter()
            .find(|as_| {
                (view_id.is_none() || view_id.unwrap() as u64 == as_.viewId.unwrap_or_default())
                    && as_.srcObjectId.unwrap_or_default() == object_id as u64
            })
            .unwrap();
        let representation = adaptation_set
            .representations
            .as_ref()
            .unwrap()
            .iter()
            .find(|r| r.id.as_ref().unwrap().parse::<u8>().unwrap() == representation_id)
            .unwrap();
        let st = representation.segment_template.as_ref().unwrap();
        let media = st.media.as_ref().unwrap();
        (
            base_url
                + self
                    .resolve_url_template(
                        media,
                        &HashMap::from_iter(vec![
                            (
                                "RepresentationID",
                                representation.id.as_ref().unwrap().clone(),
                            ),
                            (
                                "Number",
                                (((frame_offset - self.period_markers.get(period_idx).unwrap())
                                    * st.timescale.unwrap()
                                    / (st.duration.unwrap() * FPS))
                                    * st.duration.unwrap()
                                    + st.startNumber.expect("start number not provided"))
                                .to_string(),
                            ),
                        ]),
                    )
                    .as_str(),
            representation.bandwidth,
        )
    }

    pub fn available_bitrates(
        &self,
        object_id: u8,
        frame_offset: u64,
        view_id: Option<u8>,
    ) -> Vec<u64> {
        let period_idx =
            match self.period_markers[..].binary_search_by(|probe| probe.cmp(&frame_offset)) {
                Ok(idx) => idx,
                Err(idx) => idx - 1,
            };

        let period = self.mpd.periods.get(period_idx).unwrap();
        let adaptation_set = period
            .adaptations
            .as_ref()
            .unwrap()
            .iter()
            .find(|as_| {
                (view_id.is_none() || view_id.unwrap() as u64 == as_.viewId.unwrap_or_default())
                    && as_.srcObjectId.unwrap_or_default() == object_id as u64
            })
            .unwrap();
        adaptation_set
            .representations
            .as_ref()
            .unwrap()
            .iter()
            .map(|r| r.bandwidth.unwrap())
            .collect()
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
                let padded = format!("{s:0<9}");
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
                secs += hours * 60 * 60;
            }
            if let Some(s) = m.name("days") {
                let days = s.as_str().parse::<u64>().unwrap();
                secs += days * 60 * 60 * 24;
            }
            if let Some(s) = m.name("weeks") {
                let weeks = s.as_str().parse::<u64>().unwrap();
                secs += weeks * 60 * 60 * 24 * 7;
            }
            if let Some(s) = m.name("months") {
                let months = s.as_str().parse::<u64>().unwrap();
                secs += months * 60 * 60 * 24 * 30;
            }
            if let Some(s) = m.name("years") {
                let years = s.as_str().parse::<u64>().unwrap();
                secs += years * 60 * 60 * 24 * 365;
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

// Deserialize an optional XML duration string to an Option<Duration>. This is a little trickier
// than deserializing a required field with serde.
fn deserialize_xs_duration<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
where
    D: de::Deserializer<'de>,
{
    match <Option<String>>::deserialize(deserializer) {
        Ok(optstring) => match optstring {
            Some(xs) => match parse_xs_duration(&xs) {
                Ok(d) => Ok(Some(d)),
                Err(e) => Err(de::Error::custom(e)),
            },
            None => Ok(None),
        },
        // the field isn't present, return an Ok(None)
        Err(_) => Ok(None),
    }
}

fn serialize_xs_duration<S>(oxs: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // this is a very simple-minded way of converting to an ISO 8601 duration
    if let Some(xs) = oxs {
        let secs = xs.as_secs();
        let ms = xs.subsec_millis();
        serializer.serialize_str(&format!("PT{secs}.{ms:03}S"))
    } else {
        // in fact this won't be called because of the #[skip_serializing_none] annotation
        serializer.serialize_none()
    }
}

/// A URI string that specifies one or more common locations for Segments and other resources.
#[skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct BaseURL {
    #[serde(rename = "$value")]
    pub base: String,
}
/// Allows template-based `SegmentURL` construction. Specifies various substitution rules using
/// dynamic values such as `$Time$` and `$Number$` that map to a sequence of Segments.
#[skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[serde(default)]
pub(super) struct SegmentTemplate {
    pub media: Option<String>,
    pub startNumber: Option<u64>,
    // note: the spec says this is an unsigned int, not an xs:duration. In practice, some manifests
    // use a floating point value (eg.
    // https://dash.akamaized.net/akamai/bbb_30fps/bbb_with_multiple_tiled_thumbnails.mpd)
    pub duration: Option<u64>,
    pub timescale: Option<u64>,
}

/// A representation describes a version of the content, using a specific encoding and bitrate.
/// Streams often have multiple representations with different bitrates, to allow the client to
/// select that most suitable to its network conditions.
#[skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[serde(default)]
pub(super) struct Representation {
    // no id for a linked Representation (with xlink:href)
    pub id: Option<String>,
    // The specification says that @mimeType is mandatory, but it's not always present on
    // akamaized.net MPDs
    pub mimeType: Option<String>,
    /// An RFC6381 string, <https://tools.ietf.org/html/rfc6381>
    pub codecs: Option<String>,
    pub contentType: Option<String>,
    pub frameRate: Option<String>, // can be something like "15/2"
    pub bandwidth: Option<u64>,
    // pub width: Option<u64>,
    // pub height: Option<u64>,
    // pub BaseURL: Option<String>,
    #[serde(rename = "SegmentTemplate")]
    pub segment_template: Option<SegmentTemplate>,
}

/// Contains a set of Representations. For example, if multiple language streams are available for
/// the audio content, each one can be in its own AdaptationSet.
#[skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[serde(default)]
pub(super) struct AdaptationSet {
    pub id: Option<u64>,
    pub BaseURL: Option<BaseURL>,
    // eg "video/mp4"
    pub mimeType: Option<String>,
    #[serde(rename = "Representation")]
    pub representations: Option<Vec<Representation>>,
    pub viewId: Option<u64>,
    pub srcObjectId: Option<u64>,
}

/// Describes a chunk of the content with a start time and a duration. Content can be split up into
/// multiple periods (such as chapters, advertising segments).
#[skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[serde(default)]
pub(super) struct Period {
    pub id: Option<String>,
    pub start: Option<String>,
    // note: the spec says that this is an xs:duration, not an unsigned int as for other "duration" fields
    #[serde(deserialize_with = "deserialize_xs_duration", default)]
    #[serde(serialize_with = "serialize_xs_duration")]
    pub duration: Option<Duration>,
    #[serde(rename = "AdaptationSet")]
    pub adaptations: Option<Vec<AdaptationSet>>,
}

/// The root node of a parsed DASH MPD manifest.
#[skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[serde(default)]
pub(super) struct MPD {
    #[serde(rename = "type")]
    pub mpdtype: Option<String>,
    pub xmlns: Option<String>,
    pub profiles: Option<String>,
    #[serde(deserialize_with = "deserialize_xs_duration", default)]
    #[serde(serialize_with = "serialize_xs_duration")]
    pub minBufferTime: Option<Duration>,
    #[serde(deserialize_with = "deserialize_xs_duration", default)]
    #[serde(serialize_with = "serialize_xs_duration")]
    pub suggestedPresentationDelay: Option<Duration>,
    #[serde(rename = "Period", default)]
    pub periods: Vec<Period>,
    /// There may be several BaseURLs, for redundancy (for example multiple CDNs)
    #[serde(rename = "BaseURL")]
    pub base_urls: Option<Vec<BaseURL>>,
}

impl MPD {
    pub(super) fn from_xml(xml: &str) -> Result<MPD> {
        quick_xml::de::from_str(xml).map_err(|e| e.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_run() {
        let mpd = MPD::from_xml(
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
        )
        .unwrap();
        assert_eq!(
            mpd.base_urls.unwrap().get(0).unwrap().base,
            "https://www.example.com/"
        );

        let periods = mpd.periods;
        assert_eq!(periods.len(), 1);

        let first_period = periods.get(0).unwrap();
        assert_eq!(first_period.duration, Some(Duration::from_secs(120)));
        let ads = first_period.adaptations.as_ref().unwrap();
        assert_eq!(ads.len(), 2);

        let first_ad = ads.get(1).unwrap();
        assert_eq!(first_ad.id, Some(2));
        let reprs = first_ad.representations.as_ref().unwrap();
        assert_eq!(reprs.len(), 1);
    }

    #[test]
    pub fn test_run2() {
        let p = MPDParser::new(
            r#"<?xml version="1.0" encoding="UTF-8"?>
            <MPD format="pointcloud/pcd" type="static">
                <BaseURL>http://localhost:3000/</BaseURL>
                <Period id="1" duration="PT10S">
                    <AdaptationSet viewId="0">
                        <Representation id="0" bandwidth="13631488">
                            <SegmentTemplate media="longdress/$RepresentationID$/longdress_vox10_$Number$.ply" duration="1" timescale="30" startNumber="1051"></SegmentTemplate>
                        </Representation>
                        <Representation id="1" bandwidth="1536000">
                            <SegmentTemplate media="longdress/$RepresentationID$/longdress_vox10_$Number$.ply" duration="1" timescale="30" startNumber="1051"></SegmentTemplate>
                        </Representation>
                        <Representation id="2" bandwidth="204800">
                            <SegmentTemplate media="longdress/$RepresentationID$/longdress_vox10_$Number$.ply" duration="1" timescale="30" startNumber="1051"></SegmentTemplate>
                        </Representation>
                    </AdaptationSet>
                    <AdaptationSet id="5" viewId="5" srcObjectId="0">
                        <Representation id="1" bandwidth="100352">
                            <SegmentTemplate media="longdress/1/S26C2AIR0$RepresentationID$_F30_$Number$_5.bin" duration="30" timescale="30" startNumber="1051"></SegmentTemplate>
                        </Representation>
                        <Representation id="2" bandwidth="138240">
                            <SegmentTemplate media="longdress/2/S26C2AIR0$RepresentationID$_F30_$Number$_5.bin" duration="30" timescale="30" startNumber="1051"></SegmentTemplate>
                        </Representation>
                        <Representation id="3" bandwidth="196608">
                            <SegmentTemplate media="longdress/3/S26C2AIR0$RepresentationID$_F30_$Number$_5.bin" duration="30" timescale="30" startNumber="1051"></SegmentTemplate>
                        </Representation>
                    </AdaptationSet>
                </Period>
            </MPD>"#,
        );

        let periods = &p.mpd.periods;
        let first_period = periods.get(0).unwrap();
        assert_eq!(first_period.duration, Some(Duration::new(10, 0)));
        let ads = first_period.adaptations.as_ref().unwrap();
        assert_eq!(ads.len(), 2);

        let first_ad = ads.get(0).unwrap();
        let reprs = first_ad.representations.as_ref().unwrap();
        assert_eq!(reprs.len(), 3);
        assert_eq!(
            p.get_info(0, 2, 29, None),
            (
                p.get_base_url() + "longdress/2/longdress_vox10_1080.ply",
                Some(204800)
            )
        );
        assert_eq!(
            p.get_info(0, 2, 29, Some(5)),
            (
                p.get_base_url() + "longdress/2/S26C2AIR02_F30_1051_5.bin",
                Some(138240)
            )
        );
        assert_eq!(
            p.get_info(0, 2, 30, Some(5)),
            (
                p.get_base_url() + "longdress/2/S26C2AIR02_F30_1081_5.bin",
                Some(138240)
            )
        );
        assert_eq!(
            p.available_bitrates(0, 30, None),
            vec![13631488, 1536000, 204800]
        );
        assert_eq!(
            p.available_bitrates(0, 30, Some(5)),
            vec![100352, 138240, 196608]
        );
    }
}
