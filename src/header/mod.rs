use crate::{version::Version, Comments};

#[cfg(doc)]
use crate::prelude::ANTEX;

use std::collections::HashMap;

mod parsing;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// PCV compensation description
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PcvCompensation {
    /// Program used for PCVs evaluation and compensation
    pub program: String,

    /// Constellation to which this compensation applies to
    pub constellation: Constellation,

    /// URL: source of corrections
    pub url: String,
}

/// [ANTEX] file [Header].
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Header {
    /// File [Version]
    pub version: Version,

    /// Comments found in this section
    pub comments: Comments,

    /// Possible calibration method
    pub calibration_method: CalibrationMethod,

    /// Possible software/calibration operator (usually, the name of the agency).
    pub operator: Option<String>,

    /// Posible date & time of this calibration
    pub calibration_datetime: Option<Epoch>,

    /// Possible calibration date
    pub calibration_validity: CalibrationValidity,

    /// Possible file license
    pub license: Option<String>,

    /// Possible Digital Object Identifier
    pub doi: Option<String>,

    /// Type of Phase Center Variation in use
    pub pcv: pcv::Pcv,

    /// Optionnal reference antenna Serial Number
    /// used to produce this calibration file
    pub reference_ant_sn: Option<String>,

    /// Possible information about satellite vehicle antenna.
    /// This only exists in ANTEX format.
    #[cfg_attr(feature = "serde", serde(default))]
    pub sv_antenna: Option<SvAntenna>,

    /// Possible PCVs compensation information
    pub pcv_compensations: Vec<PcvCompensation>,
}

impl Default for Header {
    /// Creates a new v1.4 ANTEX [Header]
    fn default() -> Self {
        Self {
            version: Version::new(1, 4),
            program: Some(format!(
                "nav-sls/antex v{}",
                Self::format_pkg_version(env!("CARGO_PKG_VERSION"))
            )),
            pcv_compensations: Default::default(),
        }
    }
}

impl Header {
    /// Formats the package version (possibly shortenned, in case of lengthy release)
    /// to fit within a formatted COMMENT
    pub(crate) fn format_pkg_version(version: &str) -> String {
        version
            .split('.')
            .enumerate()
            .filter_map(|(nth, v)| {
                if nth < 2 {
                    Some(v.to_string())
                } else if nth == 2 {
                    Some(
                        v.split('-')
                            .filter_map(|v| {
                                if v == "rc" {
                                    Some("rc".to_string())
                                } else {
                                    let mut s = String::new();
                                    s.push_str(&v[0..1]);
                                    Some(s)
                                }
                            })
                            .join(""),
                    )
                } else {
                    None
                }
            })
            .join(".")
    }

    /// Define the type of [PCV] to be find in the following content
    pub fn with_phase_center_variation(&self, pcv: PCV) -> Self {
        let mut s = self.clone();
        s.phase_center_variations = pcv;
        s
    }

    /// Define the serial number of reference antenna
    pub fn with_reference_antenna_serial_number(&self, serial_num: &str) -> Self {
        let mut s = self.clone();
        s.reference_antenna_serial_number = Some(serial_num.to_string());
        s
    }

    /// Generates the special "FILE MERGE" comment
    pub(crate) fn merge_comment(pkg_version: &str, timestamp: Epoch) -> String {
        let formatted_version = Self::format_pkg_version(pkg_version);

        let (y, m, d, hh, mm, ss, _) = timestamp.to_gregorian_utc();

        format!(
            "nav-sls/antex v{} {:>width$}          {}{:02}{:02} {:02}{:02}{:02} {:x}",
            formatted_version,
            "FILE MERGE",
            y,
            m,
            d,
            hh,
            mm,
            ss,
            timestamp.time_scale,
            width = 19 - formatted_version.len(),
        )
    }

    /// Copies and returns [Header] with specific RINEX [Version]
    pub fn with_version(&self, version: Version) -> Self {
        let mut s = self.clone();
        s.version = version;
        s
    }
}

#[cfg(test)]
mod test {
    use crate::prelude::{Epoch, Header};
    use std::str::FromStr;

    #[test]
    fn test_merge_comment() {
        let j2000 = Epoch::from_str("2000-01-01T00:00:00 UTC").unwrap();

        for (pkg_version, formatted, comment) in [
            (
                "1.0.0",
                "1.0.0",
                "rs-rinex v1.0.0     FILE MERGE          20000101 000000 UTC",
            ),
            (
                "10.0.0",
                "10.0.0",
                "rs-rinex v10.0.0    FILE MERGE          20000101 000000 UTC",
            ),
            (
                "0.17.0",
                "0.17.0",
                "rs-rinex v0.17.0    FILE MERGE          20000101 000000 UTC",
            ),
            (
                "0.17.1",
                "0.17.1",
                "rs-rinex v0.17.1    FILE MERGE          20000101 000000 UTC",
            ),
            (
                "0.17.1-alpha",
                "0.17.1a",
                "rs-rinex v0.17.1a   FILE MERGE          20000101 000000 UTC",
            ),
            (
                "0.17.1-rc",
                "0.17.1rc",
                "rs-rinex v0.17.1rc  FILE MERGE          20000101 000000 UTC",
            ),
            (
                "0.17.1-rc-1",
                "0.17.1rc1",
                "rs-rinex v0.17.1rc1 FILE MERGE          20000101 000000 UTC",
            ),
        ] {
            assert_eq!(Header::format_pkg_version(pkg_version), formatted);

            let generated = Header::merge_comment(pkg_version, j2000);
            assert_eq!(generated, comment,);
        }
    }
}
