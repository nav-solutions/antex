#![doc(
    html_logo_url = "https://raw.githubusercontent.com/nav-solutions/.github/master/logos/logo2.jpg"
)]
#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(clippy::type_complexity)]

/*
 * ANTEX is part of the nav-solutions framework.
 * Authors: Guillaume W. Bres <guillaume.bressaix@gmail.com> et al.
 * (cf. https://github.com/nav-solutions/antex/graphs/contributors)
 * This framework is shipped under Mozilla Public V2 license.
 *
 * Documentation: https://github.com/nav-solutions/antex
 */

extern crate num_derive;

#[macro_use]
extern crate lazy_static;

#[cfg(feature = "serde")]
#[macro_use]
extern crate serde;

extern crate gnss_rs as gnss;
extern crate num;

mod header;
mod pcv;
mod record;
mod version;

pub mod error;

mod epoch;

// #[cfg(test)]
// mod tests;

use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    path::Path,
    str::FromStr,
};

use itertools::Itertools;

/// Comments identified during parsing process.
pub type Comments = Vec<String>;

use antex::{Antenna, FrequencyDependentData};

#[cfg(feature = "antex")]
use antex::{AntennaMatcher, AntennaSpecific};

#[cfg(feature = "flate2")]
use flate2::{read::GzDecoder, write::GzEncoder, Compression as GzCompression};

#[cfg(feature = "clock")]
use std::collections::BTreeMap;

pub mod prelude {
    // export
    pub use crate::{pcv::PCV, ANTEX};

    // pub re-export
    pub use gnss::prelude::{Constellation, DOMESTrackingPoint, COSPAR, DOMES, SV};
}

/// Returns true if provided line matches a standard COMMENT.
pub(crate) fn is_comment(content: &str) -> bool {
    content.len() > 60 && content.trim_end().ends_with("COMMENT")
}

/// Format according to the ANTEX standard
pub(crate) fn fmt_antex(content: &str, marker: &str) -> String {
    if content.len() < 60 {
        format!("{:<padding$}{}", content, marker, padding = 60)
    } else {
        let mut string = String::new();
        let nb_lines = num_integer::div_ceil(content.len(), 60);
        for i in 0..nb_lines {
            let start_off = i * 60;
            let end_off = std::cmp::min(start_off + 60, content.len());
            let chunk = &content[start_off..end_off];
            string.push_str(&format!("{:<padding$}{}", chunk, marker, padding = 60));
            if i < nb_lines - 1 {
                string.push('\n');
            }
        }
        string
    }
}

/// Format a standardized COMMENT, possibly wrapped on several lines.
pub(crate) fn fmt_comment(content: &str) -> String {
    fmt_rinex(content, "COMMENT")
}

#[derive(Clone, Debug)]
/// [ANTEX] comprises a [Header] and a [Record] section.
pub struct ANTEX {
    /// [Header] gives general information and describes following content.
    pub header: Header,

    /// [Comments] stored as they appeared in file body
    pub comments: Comments,

    /// [Record] is the actual file content and is heavily [RinexType] dependent
    pub record: Record,
}

impl ANTEX {
    pub fn new(header: Header, record: record::Record) -> Self {
        Self {
            header,
            record,
            comments: Comments::new(),
        }
    }

    /// Copy and return this [ANTEX] with updated [Header].
    pub fn with_header(&self, header: Header) -> Self {
        Self {
            header,
            record: self.record.clone(),
            comments: self.comments.clone(),
        }
    }

    /// Replace [Header] with mutable access.
    pub fn replace_header(&mut self, header: Header) {
        self.header = header.clone();
    }

    /// Copy and return this [ANTEX] with updated [Record]
    pub fn with_record(&self, record: Record) -> Self {
        Self {
            record,
            header: self.header.clone(),
            comments: self.comments.clone(),
        }
    }

    /// Replace [Record] with mutable access.
    pub fn replace_record(&mut self, record: Record) {
        self.record = record.clone();
    }

    /// Parse [ANTEX] content by consuming [BufReader] (efficient buffered reader).
    /// Attributes potentially described by a file name need to be provided either
    /// manually / externally, or guessed when parsing has been completed.
    pub fn parse<R: Read>(reader: &mut BufReader<R>) -> Result<Self, ParsingError> {
        // Parses Header section (=consumes header until this point)
        let mut header = Header::parse(reader)?;

        // Parse record (=consumes rest of this resource)
        // Comments are preserved and store "as is"
        let (record, comments) = Record::parse(&mut header, reader)?;

        Ok(Self {
            header,
            comments,
            record,
        })
    }

    /// Format [ANTEX] into writable I/O using efficient buffered writer
    /// and following standard specifications. The revision to be followed is defined
    /// in [Header] section. This is the mirror operation of [Self::parse].
    pub fn format<W: Write>(&self, writer: &mut BufWriter<W>) -> Result<(), FormattingError> {
        self.header.format(writer)?;
        self.record.format(writer, &self.header)?;
        writer.flush()?;
        Ok(())
    }

    /// Parses [ANTEX] object from local readable file.
    ///
    /// Will panic if provided file does not exist or is not readable.
    /// See [Self::from_gzip_file] for seamless Gzip support.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ParsingError> {
        let path = path.as_ref();

        let fd = File::open(path)?;

        let mut reader = BufReader::new(fd);
        let mut antex = Self::parse(&mut reader)?;
        Ok(antex)
    }

    /// Dumps [ANTEX] into writable local file (as readable ASCII UTF-8)
    /// using efficient buffered formatting.
    ///
    /// This is the mirror operation of [Self::from_file].
    /// Returns total amount of bytes that was generated.
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), FormattingError> {
        let fd = File::create(path)?;
        let mut writer = BufWriter::new(fd);
        self.format(&mut writer)?;
        Ok(())
    }

    /// Parses [ANTEX] from local gzip compressed file.
    /// ```
    /// ```
    #[cfg(feature = "flate2")]
    #[cfg_attr(docsrs, doc(cfg(feature = "flate2")))]
    pub fn from_gzip_file<P: AsRef<Path>>(path: P) -> Result<Rinex, ParsingError> {
        let path = path.as_ref();

        let fd = File::open(path)?;

        let reader = GzDecoder::new(fd);
        let mut reader = BufReader::new(reader);
        let mut rinex = Self::parse(&mut reader)?;
        Ok(rinex)
    }

    /// Dumps and gzip encodes [ANTEX] into writable local file,
    /// using efficient buffered formatting.
    ///
    /// This is the mirror operation of [Self::from_gzip_file].
    #[cfg(feature = "flate2")]
    #[cfg_attr(docsrs, doc(cfg(feature = "flate2")))]
    pub fn to_gzip_file<P: AsRef<Path>>(&self, path: P) -> Result<(), FormattingError> {
        let fd = File::create(path)?;
        let compression = GzCompression::new(5);
        let mut writer = BufWriter::new(GzEncoder::new(fd, compression));
        self.format(&mut writer)?;
        Ok(())
    }

    /// Determines whether this [ANTEX] is the result of a previous file merge operations.
    /// That is, the combination of at least two files merged together.  
    /// This is determined by the presence of custom yet somewhat standardized header comment.
    pub fn is_merged(&self) -> bool {
        let special_comment = String::from("FILE MERGE");
        for comment in self.header.comments.iter() {
            if comment.contains(&special_comment) {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{fmt_comment, is_comment};

    #[test]
    fn fmt_comments_singleline() {
        for desc in [
            "test",
            "just a basic comment",
            "just another lengthy comment blahblabblah",
        ] {
            let comment = fmt_comment(desc);

            assert!(
                comment.len() >= 60,
                "comments should be at least 60 byte long"
            );

            assert_eq!(
                comment.find("COMMENT"),
                Some(60),
                "comment marker should located @ 60"
            );

            assert!(is_comment(&comment), "should be valid comment");
        }
    }

    #[test]
    fn fmt_wrapped_comments() {
        for desc in ["just trying to form a very lengthy comment that will overflow since it does not fit in a single line",
            "just trying to form a very very lengthy comment that will overflow since it does fit on three very meaningful lines. Imazdmazdpoakzdpoakzpdokpokddddddddddddddddddaaaaaaaaaaaaaaaaaaaaaaa"] {
            let nb_lines = num_integer::div_ceil(desc.len(), 60);
            let comments = fmt_comment(desc);
            assert_eq!(comments.lines().count(), nb_lines);
            for line in comments.lines() {
                assert!(line.len() >= 60, "comment line should be at least 60 byte long");
                assert_eq!(line.find("COMMENT"), Some(60), "comment marker should located @ 60");
                assert!(is_comment(line), "should be valid comment");
            }
        }
    }
}
