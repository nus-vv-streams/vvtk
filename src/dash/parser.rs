use anyhow::{Context, Result};
use roxmltree::{Document, Node};

/* PCC-Dash
<?xml version='1.0'?>
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
            <Representation id="1" bandwidth="3500000">
                <SegmentTemplate
                    media="redandblack/1/segment_$Number%04u$.bin"
                    duration="30" timescale="30" startNumber="1"/>
            </Representation>
            <!-- further representations -->
        </AdaptationSet>
        <!-- further adaptation sets -->
    </Period>
    <!-- further periods -->
</MPD>
*/
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

    pub fn get_segment_templates(&'a self, representation: &Node<'a, 'a>) -> Vec<Node<'a, 'a>> {
        representation
            .children()
            .filter(|n| n.has_tag_name("SegmentTemplate"))
            .collect()
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
}
