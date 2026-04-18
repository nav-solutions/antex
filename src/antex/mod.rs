//! Antex (ATX) - special RINEX, for antenna caracteristics
pub mod antenna;
pub mod frequency;
pub mod pcv;
pub mod record;

pub use pcv::Pcv;
// pub use frequency::{Frequency, Pattern};

pub use antenna::{
    Antenna, AntennaMatcher, AntennaSpecific, Calibration, CalibrationMethod, RxAntenna, SvAntenna,
};

pub use record::{FrequencyDependentData, Record};

use crate::prelude::FormattingError;

use std::io::{BufWriter, Write};
