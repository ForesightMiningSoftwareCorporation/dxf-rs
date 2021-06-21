// other implementation is in `generated/header.rs`

use std::io::Write;

use crate::code_pair_put_back::CodePairPutBack;
use crate::code_pair_writer::CodePairWriter;
use crate::enums::*;
use crate::helper_functions::*;
use crate::{CodePair, DxfError, DxfResult};

pub use crate::generated::header::*;

impl Header {
    /// Ensure all values are valid.
    pub fn normalize(&mut self) {
        ensure_positive_or_default(&mut self.default_text_height, 0.2);
        ensure_positive_or_default(&mut self.trace_width, 0.05);
        default_if_empty(&mut self.text_style, "STANDARD");
        default_if_empty(&mut self.current_layer, "0");
        default_if_empty(&mut self.current_entity_line_type, "BYLAYER");
        default_if_empty(&mut self.dimension_style_name, "STANDARD");
        default_if_empty(&mut self.file_name, ".");
    }
    pub(crate) fn read(iter: &mut CodePairPutBack) -> DxfResult<Header> {
        let mut header = Header::default();
        loop {
            match iter.next() {
                Some(Ok(pair)) => {
                    match pair.code {
                        0 => {
                            iter.put_back(Ok(pair));
                            break;
                        }
                        9 => {
                            let last_header_variable = pair.assert_string()?;
                            loop {
                                match iter.next() {
                                    Some(Ok(pair)) => {
                                        if pair.code == 0 || pair.code == 9 {
                                            // ENDSEC or a new header variable
                                            iter.put_back(Ok(pair));
                                            break;
                                        } else {
                                            header
                                                .set_header_value(&last_header_variable, &pair)?;
                                            if last_header_variable == "$ACADVER"
                                                && header.version >= AcadVersion::R2007
                                            {
                                                iter.read_as_utf8();
                                            }
                                        }
                                    }
                                    Some(Err(e)) => return Err(e),
                                    None => break,
                                }
                            }
                        }
                        _ => return Err(DxfError::UnexpectedCodePair(pair, String::from(""))),
                    }
                }
                Some(Err(e)) => return Err(e),
                None => break,
            }
        }

        Ok(header)
    }
    pub(crate) fn write<T>(&self, writer: &mut CodePairWriter<T>) -> DxfResult<()>
    where
        T: Write + ?Sized,
    {
        writer.write_code_pair(&CodePair::new_str(0, "SECTION"))?;
        writer.write_code_pair(&CodePair::new_str(2, "HEADER"))?;
        self.write_code_pairs(writer)?;
        writer.write_code_pair(&CodePair::new_str(0, "ENDSEC"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::entities::*;
    use crate::enums::*;
    use crate::helper_functions::tests::*;
    use crate::*;
    use std::time::Duration;

    #[test]
    fn empty_header() {
        let _file = drawing_from_pairs(vec![
            CodePair::new_str(0, "SECTION"),
            CodePair::new_str(2, "HEADER"),
            CodePair::new_str(0, "ENDSEC"),
            CodePair::new_str(0, "EOF"),
        ]);
    }

    #[test]
    fn specific_header_values() {
        let file = from_section_pairs(
            "HEADER",
            vec![
                CodePair::new_str(9, "$ACADMAINTVER"),
                CodePair::new_i16(70, 16),
                CodePair::new_str(9, "$ACADVER"),
                CodePair::new_str(1, "AC1012"),
                CodePair::new_str(9, "$ANGBASE"),
                CodePair::new_f64(50, 55.0),
                CodePair::new_str(9, "$ANGDIR"),
                CodePair::new_i16(70, 1),
                CodePair::new_str(9, "$ATTMODE"),
                CodePair::new_i16(70, 1),
                CodePair::new_str(9, "$AUNITS"),
                CodePair::new_i16(70, 3),
                CodePair::new_str(9, "$AUPREC"),
                CodePair::new_i16(70, 7),
                CodePair::new_str(9, "$CLAYER"),
                CodePair::new_str(8, "<current layer>"),
                CodePair::new_str(9, "$LUNITS"),
                CodePair::new_i16(70, 6),
                CodePair::new_str(9, "$LUPREC"),
                CodePair::new_i16(70, 7),
            ],
        );
        assert_eq!(16, file.header.maintenance_version);
        assert_eq!(AcadVersion::R13, file.header.version);
        assert!(approx_eq!(f64, 55.0, file.header.angle_zero_direction));
        assert_eq!(AngleDirection::Clockwise, file.header.angle_direction);
        assert_eq!(
            AttributeVisibility::Normal,
            file.header.attribute_visibility
        );
        assert_eq!(AngleFormat::Radians, file.header.angle_unit_format);
        assert_eq!(7, file.header.angle_unit_precision);
        assert_eq!("<current layer>", file.header.current_layer);
        assert_eq!(UnitFormat::Architectural, file.header.unit_format);
        assert_eq!(7, file.header.unit_precision);
    }

    #[test]
    fn read_alternate_version() {
        let file = from_section(
            "HEADER",
            vec!["  9", "$ACADVER", "  1", "15.05"]
                .join("\r\n")
                .as_str(),
        );
        assert_eq!(AcadVersion::R2000, file.header.version);
    }

    #[test]
    fn read_invalid_version() {
        let file = from_section(
            "HEADER",
            vec!["  9", "$ACADVER", "  1", "AC3.14159"]
                .join("\r\n")
                .as_str(),
        );
        assert_eq!(AcadVersion::R12, file.header.version);
    }

    #[test]
    fn read_multi_value_variable() {
        let file = from_section(
            "HEADER",
            vec!["9", "$EXTMIN", "10", "1.1", "20", "2.2", "30", "3.3"]
                .join("\r\n")
                .as_str(),
        );
        assert_eq!(
            Point::new(1.1, 2.2, 3.3),
            file.header.minimum_drawing_extents
        )
    }

    #[test]
    fn write_multiple_value_variable() {
        let mut file = Drawing::new();
        file.header.minimum_drawing_extents = Point::new(1.1, 2.2, 3.3);
        assert!(to_test_string(&file).contains(
            vec!["9", "$EXTMIN", " 10", "1.1", " 20", "2.2", " 30", "3.3"]
                .join("\r\n")
                .as_str()
        ));
    }

    #[test]
    fn normalize_header() {
        let mut header = Header::default();
        header.default_text_height = -1.0; // $TEXTSIZE; normalized to 0.2
        header.trace_width = 0.0; // $TRACEWID; normalized to 0.05
        header.text_style = String::new(); // $TEXTSTYLE; normalized to "STANDARD"
        header.current_layer = String::new(); // $CLAYER; normalized to "0"
        header.current_entity_line_type = String::new(); // $CELTYPE; normalized to "BYLAYER"
        header.dimension_style_name = String::new(); // $DIMSTYLE; normalized to "STANDARD"
        header.file_name = String::new(); // $MENU; normalized to "."
        header.normalize();
        assert!(approx_eq!(f64, 0.2, header.default_text_height));
        assert!(approx_eq!(f64, 0.05, header.trace_width));
        assert_eq!("STANDARD", header.text_style);
        assert_eq!("0", header.current_layer);
        assert_eq!("BYLAYER", header.current_entity_line_type);
        assert_eq!("STANDARD", header.dimension_style_name);
        assert_eq!(".", header.file_name);
    }

    #[test]
    fn read_header_flags() {
        let file = from_section(
            "HEADER",
            vec!["9", "$OSMODE", "70", "12"].join("\r\n").as_str(),
        );
        assert!(!file.header.get_end_point_snap());
        assert!(!file.header.get_mid_point_snap());
        assert!(file.header.get_center_snap());
        assert!(file.header.get_node_snap());
        assert!(!file.header.get_quadrant_snap());
        assert!(!file.header.get_intersection_snap());
        assert!(!file.header.get_insertion_snap());
        assert!(!file.header.get_perpendicular_snap());
        assert!(!file.header.get_tangent_snap());
        assert!(!file.header.get_nearest_snap());
        assert!(!file.header.get_apparent_intersection_snap());
        assert!(!file.header.get_extension_snap());
        assert!(!file.header.get_parallel_snap());
    }

    #[test]
    fn write_header_flags() {
        let mut file = Drawing::new();
        file.header.set_end_point_snap(false);
        file.header.set_mid_point_snap(false);
        file.header.set_center_snap(true);
        file.header.set_node_snap(true);
        file.header.set_quadrant_snap(false);
        file.header.set_intersection_snap(false);
        file.header.set_insertion_snap(false);
        file.header.set_perpendicular_snap(false);
        file.header.set_tangent_snap(false);
        file.header.set_nearest_snap(false);
        file.header.set_apparent_intersection_snap(false);
        file.header.set_extension_snap(false);
        file.header.set_parallel_snap(false);
        assert_contains(&file, vec!["  9", "$OSMODE", " 70", "    12"].join("\r\n"));
    }

    #[test]
    fn read_variable_with_different_codes() {
        // read $CMLSTYLE as code 7
        let file = from_section(
            "HEADER",
            vec!["  9", "$CMLSTYLE", "  7", "cml-style-7"]
                .join("\r\n")
                .as_str(),
        );
        assert_eq!("cml-style-7", file.header.current_multiline_style);

        // read $CMLSTYLE as code 2
        let file = from_section(
            "HEADER",
            vec!["  9", "$CMLSTYLE", "  2", "cml-style-2"]
                .join("\r\n")
                .as_str(),
        );
        assert_eq!("cml-style-2", file.header.current_multiline_style);
    }

    #[test]
    fn write_variable_with_different_codes() {
        // R13 writes $CMLSTYLE as a code 7
        let mut file = Drawing::new();
        file.header.version = AcadVersion::R13;
        file.header.current_multiline_style = String::from("cml-style-7");
        assert_contains(
            &file,
            vec!["  9", "$CMLSTYLE", "  7", "cml-style-7"].join("\r\n"),
        );

        // R14+ writes $CMLSTYLE as a code 2
        let mut file = Drawing::new();
        file.header.version = AcadVersion::R14;
        file.header.current_multiline_style = String::from("cml-style-2");
        assert_contains(
            &file,
            vec!["  9", "$CMLSTYLE", "  2", "cml-style-2"].join("\r\n"),
        );
    }

    #[test]
    fn read_drawing_edit_duration() {
        let file = from_section(
            "HEADER",
            vec!["  9", "$TDINDWG", " 40", "100.0"]
                .join("\r\n")
                .as_str(),
        );
        assert_eq!(Duration::from_secs(100), file.header.time_in_drawing);
    }

    #[test]
    fn write_proper_handseed_on_new_file() {
        let mut drawing = Drawing::new();
        drawing.add_entity(Entity::new(EntityType::Line(Line::new(
            Point::origin(),
            Point::origin(),
        ))));
        assert_contains(&drawing, vec!["  9", "$HANDSEED", "  5", "11"].join("\r\n"));
    }

    #[test]
    fn write_proper_handseed_on_read_file() {
        let mut drawing = from_section(
            "HEADER",
            vec!["  9", "$HANDSEED", "  5", "11"].join("\r\n").as_str(),
        );
        drawing.add_entity(Entity::new(EntityType::Line(Line::new(
            Point::origin(),
            Point::origin(),
        ))));
        assert_contains(&drawing, vec!["  9", "$HANDSEED", "  5", "15"].join("\r\n"));
    }

    #[test]
    fn dont_write_suppressed_variables() {
        let mut drawing = Drawing::new();
        drawing.header.version = AcadVersion::R2004;
        assert_contains(&drawing, vec!["9", "$HIDETEXT", "280"].join("\r\n"));
        assert_not_contains(&drawing, vec!["9", "$HIDETEXT", "290"].join("\r\n"));
    }
}
