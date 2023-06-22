mod acd;
mod cd;
mod cd_psnr;
mod hd;
mod lc_psnr;
mod psnr;
mod vqoe;

use std::{
    collections::BTreeMap,
    ffi::OsString,
    io::{BufRead, BufReader, Read, Write},
};

use kiddo::KdTree;

use crate::formats::{pointxyzrgba::PointXyzRgba, PointCloud};

use self::acd::Acd;
use self::cd::Cd;
use self::cd_psnr::CdPsnr;
use self::hd::Hd;
use self::lc_psnr::LcPsnr;
use self::psnr::Psnr;
use self::vqoe::VQoE;

#[derive(Debug, Clone)]
pub struct Metrics(BTreeMap<String, String>);

impl Metrics {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn insert(&mut self, key: String, value: String) {
        self.0.insert(key, value);
    }

    pub fn metrics(&self) -> Vec<(String, String)> {
        self.0
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    pub fn from_reader<R>(read: &mut R) -> Self
    where
        R: Read,
    {
        let mut map = BTreeMap::new();
        let mut s = String::new();
        let mut buf = BufReader::new(read);
        while let Ok(size) = buf.read_line(&mut s) {
            if size == 0 {
                break;
            }
            let mut split = s.split(',');
            map.insert(
                split.next().expect("Metric name expected").to_string(),
                split.next().expect("Metric value expected").to_string(),
            );
            s.clear();
        }
        Self(map)
    }
    pub fn write_to<W>(&self, writer: &mut W) -> std::io::Result<()>
    where
        W: Write,
    {
        for (key, val) in self.0.iter() {
            writeln!(writer, "{},{}", key, val)?;
        }
        Ok(())
    }
}

pub fn calculate_metrics(
    original: &PointCloud<PointXyzRgba>,
    reconstructed: &PointCloud<PointXyzRgba>,
    metrics: &Vec<OsString>,
) -> Metrics {
    let mut original_tree = KdTree::new();
    for (i, pt) in original.points.iter().enumerate() {
        original_tree
            .add(&[pt.x, pt.y, pt.z], i)
            .expect("Failed to add to original tree");
    }
    let mut reconstructed_tree = KdTree::new();
    for (i, pt) in reconstructed.points.iter().enumerate() {
        reconstructed_tree
            .add(&[pt.x, pt.y, pt.z], i)
            .expect("Failed to add to original tree");
    }

    let mut metrics_report = Metrics::new();

    let has_all = metrics.contains(&OsString::from("all"));

    let mut acd_rt: Option<f64> = None;
    let mut acd_tr: Option<f64> = None;
    if has_all | metrics.contains(&OsString::from("acd")) {
        acd_rt = Acd::calculate_if_none(
            acd_rt,
            &original.points,
            &original_tree,
            &reconstructed.points,
            &reconstructed_tree,
        );
        acd_tr = Acd::calculate_if_none(
            acd_tr,
            &reconstructed.points,
            &reconstructed_tree,
            &original.points,
            &original_tree,
        );
        metrics_report.insert(
            "acd_rt".to_string(),
            format!("{:.5}", acd_rt.clone().unwrap()),
        );
        metrics_report.insert(
            "acd_tr".to_string(),
            format!("{:.5}", acd_tr.clone().unwrap()),
        );
    }

    let mut cd: Option<f64> = None;
    if has_all || metrics.contains(&OsString::from("cd")) {
        cd = Cd::calculate_from_acd(
            acd_rt.clone(),
            acd_tr.clone(),
            &original.points,
            &original_tree,
            &reconstructed.points,
            &reconstructed_tree,
        );
        metrics_report.insert("cd".to_string(), format!("{:.5}", cd.clone().unwrap()));
    }

    // let mut cd_psnr: Option<f64> = None;
    if has_all || metrics.contains(&OsString::from("cd_psnr")) {
        let cd_psnr = CdPsnr::calculate_from_acd_or_cd(
            acd_rt.clone(),
            acd_tr.clone(),
            cd.clone(),
            &original.points,
            &original_tree,
            &reconstructed.points,
            &reconstructed_tree,
        );
        metrics_report.insert("cd_psnr".to_string(), format!("{:.5}", cd_psnr.unwrap()));
    }

    if has_all || metrics.contains(&OsString::from("hd")) {
        let hd = Hd::calculate_metric(
            &original.points,
            &original_tree,
            &reconstructed.points,
            &reconstructed_tree,
        );
        metrics_report.insert("hd".to_string(), format!("{:.5}", hd.clone()));
    }

    if has_all || metrics.contains(&OsString::from("lc_psnr")) {
        let lc_psnr = LcPsnr::calculate_metric(
            &original.points,
            &original_tree,
            &reconstructed.points,
            &reconstructed_tree,
        );
        metrics_report.insert("lc_psnr".to_string(), format!("{:.5}", lc_psnr));
    }

    if has_all || metrics.contains(&OsString::from("vqoe")) {
        let vqoe = VQoE::calculate_metric(
            &original.points,
            &original_tree,
            &reconstructed.points,
            &reconstructed_tree,
        );
        metrics_report.insert("vqoe".to_string(), format!("{:.5}", vqoe));
    }

    Psnr::calculate_metric(
        &original.points,
        &original_tree,
        &reconstructed.points,
        &reconstructed_tree,
        &mut metrics_report,
    );

    metrics_report
}
