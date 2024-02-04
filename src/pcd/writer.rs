use crate::formats::PointCloudSegment;
use crate::formats::{
    pointxyzrgba::PointXyzRgba, pointxyzrgbanormal::PointXyzRgbaNormal, PointCloud,
};
use crate::pcd::{
    PCDDataType, PCDField, PCDFieldDataType, PCDFieldSize, PCDFieldType, PCDHeader, PCDVersion,
    PointCloudData,
};
use byteorder::{NativeEndian, ReadBytesExt};
use std::fs::File;
use std::io::{BufWriter, Cursor, Write};
use std::path::Path;

type IOResult = Result<(), std::io::Error>;

/// Writes the point cloud into the file
pub fn write_pcd_file<P: AsRef<Path>>(
    pcd: &PointCloudData,
    data_type: PCDDataType,
    p: P,
) -> IOResult {
    let file = File::create(p)?;
    let writer = BufWriter::new(file);
    Writer::new(pcd, data_type, writer).write()?;
    Ok(())
}

/// Writes the point cloud into the provided writer
pub fn write_pcd<W: Write>(
    pcd: &PointCloudData,
    data_type: PCDDataType,
    writer: &mut W,
) -> IOResult {
    Writer::new(pcd, data_type, writer).write()?;
    Ok(())
}

struct Writer<'a, W: Write> {
    writer: W,
    pcd: &'a PointCloudData,
    data_type: PCDDataType,
}

impl<'a, W: Write> Writer<'a, W> {
    pub fn new(pcd: &'a PointCloudData, data_type: PCDDataType, writer: W) -> Self {
        Self {
            pcd,
            data_type,
            writer,
        }
    }

    fn write(mut self) -> IOResult {
        self.write_header()?;
        self.write_data()
    }

    fn write_header(&mut self) -> IOResult {
        let header = self.pcd.header();
        let mut fields = String::new();
        let mut sizes = String::new();
        let mut types = String::new();
        let mut counts = String::new();

        for field in header.fields() {
            fields.push_str(field.name());
            fields.push(' ');

            let size: PCDFieldSize = field.data_type().into();
            sizes.push_str(&size.to_string());
            sizes.push(' ');

            let field_type: PCDFieldType = field.data_type().into();
            types.push_str(&field_type.to_string());
            types.push(' ');

            counts.push_str(&field.count().to_string());
            counts.push(' ');
        }
        // Remove last whitespace
        fields.pop();
        sizes.pop();
        types.pop();
        counts.pop();

        let viewpoint = header.viewpoint();
        let viewpoint_str = format!(
            "{} {} {} {} {} {} {}",
            viewpoint[0],
            viewpoint[1],
            viewpoint[2],
            viewpoint[3],
            viewpoint[4],
            viewpoint[5],
            viewpoint[6]
        );

        let header_str = format!(
            "VERSION {}\n\
            FIELDS {}\n\
            SIZE {}\n\
            TYPE {}\n\
            COUNT {}\n\
            WIDTH {}\n\
            HEIGHT {}\n\
            VIEWPOINT {}\n\
            POINTS {}\n\
            DATA {}\n",
            header.version().to_string(),
            fields,
            sizes,
            types,
            counts,
            header.width(),
            header.height(),
            viewpoint_str,
            header.points(),
            self.data_type.to_string()
        );

        self.writer.write_all(header_str.as_bytes())?;
        Ok(())
    }

    fn write_data(&mut self) -> IOResult {
        match self.data_type {
            PCDDataType::Ascii => self.write_ascii(),
            PCDDataType::Binary => self.write_binary(),
            PCDDataType::CompressedBinary => panic!("Write compressed binary not supported"),
        }
    }

    fn write_ascii(&mut self) -> IOResult {
        use PCDFieldDataType::*;

        let header = self.pcd.header();
        let mut rdr = Cursor::new(self.pcd.data());
        let mut s = String::new();
        for _ in 0..header.points() {
            for field in header.fields() {
                for _ in 0..field.count() {
                    s.push_str(&match field.data_type() {
                        U8 => rdr.read_u8()?.to_string(),
                        I8 => rdr.read_i8()?.to_string(),
                        U16 => rdr.read_u16::<NativeEndian>()?.to_string(),
                        I16 => rdr.read_i16::<NativeEndian>()?.to_string(),
                        U32 => rdr.read_u32::<NativeEndian>()?.to_string(),
                        I32 => rdr.read_i32::<NativeEndian>()?.to_string(),
                        F32 => rdr.read_f32::<NativeEndian>()?.to_string(),
                        F64 => rdr.read_f64::<NativeEndian>()?.to_string(),
                    });
                    s.push(' ');
                }
            }
            s.pop();
            self.writer.write_all(s.as_bytes())?;
            self.writer.write_all(&[b'\n'])?;
            s.clear();
        }
        Ok(())
    }

    fn write_binary(&mut self) -> IOResult {
        self.writer.write_all(self.pcd.data())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::pcd::{
        read_pcd, write_pcd, PCDDataType, PCDField, PCDFieldSize, PCDFieldType, PCDHeader,
        PCDVersion, PointCloudData,
    };
    use byteorder::{NativeEndian, WriteBytesExt};
    use image::EncodableLayout;
    use std::io::{BufReader, BufWriter};

    #[test]
    fn test_write_ascii() {
        let expected = b"VERSION .7\n\
               FIELDS x y z rgb\n\
               SIZE 4 4 4 4\n\
               TYPE F F F F\n\
               COUNT 1 1 1 1\n\
               WIDTH 1\n\
               HEIGHT 1\n\
               VIEWPOINT 0 0 0 1 0 0 0\n\
               POINTS 1\n\
               DATA ascii\n\
               25 70.3 40.4 20.1\n";

        let mut data = vec![];
        data.write_f32::<NativeEndian>(25.0).unwrap();
        data.write_f32::<NativeEndian>(70.3).unwrap();
        data.write_f32::<NativeEndian>(40.4).unwrap();
        data.write_f32::<NativeEndian>(20.1).unwrap();

        let pcd = PointCloudData::new(
            PCDHeader::new(
                PCDVersion::V0_7,
                vec![
                    PCDField::new("x".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1)
                        .unwrap(),
                    PCDField::new("y".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1)
                        .unwrap(),
                    PCDField::new("z".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1)
                        .unwrap(),
                    PCDField::new(
                        "rgb".to_string(),
                        PCDFieldSize::Four,
                        PCDFieldType::Float,
                        1,
                    )
                    .unwrap(),
                ],
                1,
                1,
                [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
                1,
                "ascii".parse().unwrap(),
            )
            .unwrap(),
            data,
        )
        .unwrap();

        let mut buf = BufWriter::new(Vec::new());
        write_pcd(&pcd, PCDDataType::Ascii, &mut buf).unwrap();
        assert_eq!(buf.into_inner().unwrap(), expected);
    }

    #[test]
    fn test_write_binary() {
        let mut data = vec![];
        data.write_f32::<NativeEndian>(25.0).unwrap();
        data.write_f32::<NativeEndian>(70.3).unwrap();
        data.write_f32::<NativeEndian>(40.4).unwrap();
        data.write_f32::<NativeEndian>(20.1).unwrap();

        let pcd = PointCloudData::new(
            PCDHeader::new(
                PCDVersion::V0_7,
                vec![
                    PCDField::new("x".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1)
                        .unwrap(),
                    PCDField::new("y".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1)
                        .unwrap(),
                    PCDField::new("z".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1)
                        .unwrap(),
                    PCDField::new(
                        "rgb".to_string(),
                        PCDFieldSize::Four,
                        PCDFieldType::Float,
                        1,
                    )
                    .unwrap(),
                ],
                1,
                1,
                [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
                1,
                "binary".parse().unwrap(),
            )
            .unwrap(),
            data,
        )
        .unwrap();

        let mut buf = BufWriter::new(Vec::new());
        write_pcd(&pcd, PCDDataType::Binary, &mut buf).unwrap();
        let vec = buf.into_inner().unwrap();
        let rdr = BufReader::new(vec.as_bytes());
        let new_pcd = read_pcd(rdr).unwrap();
        assert_eq!(new_pcd.header(), pcd.header());
        assert_eq!(new_pcd.data(), pcd.data());
    }
}

pub fn create_pcd(point_cloud: &PointCloud<PointXyzRgba>) -> PointCloudData {
    let header = PCDHeader::new(
        PCDVersion::V0_7,
        vec![
            PCDField::new("x".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1).unwrap(),
            PCDField::new("y".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1).unwrap(),
            PCDField::new("z".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1).unwrap(),
            PCDField::new(
                "rgba".to_string(),
                PCDFieldSize::Four,
                PCDFieldType::Unsigned,
                1,
            )
            .unwrap(),
        ],
        point_cloud.number_of_points as u64,
        1,
        [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
        point_cloud.number_of_points as u64,
        PCDDataType::Ascii, // this is a placeholder value, it will be overwritten accoradingly in write_pcd_file()
    )
    .unwrap();
    let bytes = unsafe {
        let mut points = std::mem::ManuallyDrop::new(point_cloud.points.clone());
        Vec::from_raw_parts(
            points.as_mut_ptr() as *mut u8,
            point_cloud.number_of_points * std::mem::size_of::<PointXyzRgba>(),
            points.capacity() * std::mem::size_of::<PointXyzRgba>(),
        )
    };
    PointCloudData::new(header, bytes).unwrap()
}

pub fn create_pcd_from_pc_segment(pc_segment: &PointCloudSegment<PointXyzRgba>) -> PointCloudData {
    let header = PCDHeader::new(
        PCDVersion::V0_7,
        vec![
            PCDField::new("x".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1).unwrap(),
            PCDField::new("y".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1).unwrap(),
            PCDField::new("z".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1).unwrap(),
            PCDField::new(
                "rgba".to_string(),
                PCDFieldSize::Four,
                PCDFieldType::Unsigned,
                1,
            )
            .unwrap(),
        ],
        pc_segment.number_of_points as u64,
        1,
        [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
        pc_segment.number_of_points as u64,
        PCDDataType::Ascii, // this is a placeholder value, it will be overwritten accoradingly in write_pcd_file()
    )
    .unwrap();
    let bytes = unsafe {
        let mut points = std::mem::ManuallyDrop::new(pc_segment.points.clone());
        Vec::from_raw_parts(
            points.as_mut_ptr() as *mut u8,
            pc_segment.number_of_points * std::mem::size_of::<PointXyzRgba>(),
            points.capacity() * std::mem::size_of::<PointXyzRgba>(),
        )
    };
    PointCloudData::new(header, bytes).unwrap()
}

pub fn create_pcd_from_pc_normal(point_cloud: &PointCloud<PointXyzRgbaNormal>) -> PointCloudData {
    let header = PCDHeader::new(
        PCDVersion::V0_7,
        vec![
            PCDField::new("x".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1).unwrap(),
            PCDField::new("y".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1).unwrap(),
            PCDField::new("z".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1).unwrap(),
            PCDField::new(
                "rgba".to_string(),
                PCDFieldSize::Four,
                PCDFieldType::Unsigned,
                1,
            )
            .unwrap(),
            PCDField::new("nx".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1).unwrap(),
            PCDField::new("ny".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1).unwrap(),
            PCDField::new("nz".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1).unwrap(),
        ],
        point_cloud.number_of_points as u64,
        1,
        [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
        point_cloud.number_of_points as u64,
        PCDDataType::Ascii, // This is a placeholder value, it will be overwritten accordingly in write_pcd_file()
    )
    .unwrap();

    let bytes = unsafe {
        let mut points = std::mem::ManuallyDrop::new(point_cloud.points.clone());
        Vec::from_raw_parts(
            points.as_mut_ptr() as *mut u8,
            point_cloud.number_of_points * std::mem::size_of::<PointXyzRgbaNormal>(),
            points.capacity() * std::mem::size_of::<PointXyzRgbaNormal>(),
        )
    };

    PointCloudData::new(header, bytes).unwrap()
}
