//! ANTEX file header parsing
use crate::{
    antex::{HeaderFields as AntexHeader, Pcv},
    hardware::{Antenna, Receiver, SvAntenna},
    header::{DcbCompensation, Header, PcvCompensation},
    leap::Leap,
    linspace::Linspace,
    marker::{GeodeticMarker, MarkerType},
    prelude::{Constellation, Duration, Epoch, ParsingError, TimeScale, COSPAR, DOMES, SV},
    types::Type,
    version::Version,
};

use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Read},
    str::FromStr,
};

impl Header {
    /// Parse [Header] by consuming [BufReader] until end of this section
    pub fn parse<R: Read>(reader: &mut BufReader<R>) -> Result<Self, ParsingError> {
        let mut rinex_type = Type::default();
        let mut version = Version::default();
        let mut constellation: Option<Constellation> = None;

        let mut program = Option::<String>::None;
        let mut run_by = Option::<String>::None;
        let mut date = Option::<String>::None;
        let mut observer = Option::<String>::None;
        let mut agency = Option::<String>::None;
        let mut license = Option::<String>::None;
        let mut doi = Option::<String>::None;
        let mut station_url = Option::<String>::None;
        let mut geodetic_marker = Option::<GeodeticMarker>::None;
        let mut cospar = Option::<COSPAR>::None;
        let mut glo_channels: HashMap<SV, i8> = HashMap::new();
        let mut rcvr: Option<Receiver> = None;
        let mut rcvr_antenna: Option<Antenna> = None;
        let mut sv_antenna: Option<SvAntenna> = None;
        let mut leap: Option<Leap> = None;
        let mut sampling_interval: Option<Duration> = None;

        let mut rx_position: Option<_> = Option::<(f64, f64, f64)>::None;

        let mut dcb_compensations: Vec<DcbCompensation> = Vec::new();
        let mut pcv_compensations: Vec<PcvCompensation> = Vec::new();

        let mut comments = Vec::<String>::with_capacity(8);

        // RINEX specific fields
        let mut current_constell: Option<Constellation> = None;

        for line in reader.lines() {
            if line.is_err() {
                continue;
            }

            let line = line.unwrap();

            if line.len() < 60 {
                continue; // --> invalid header content
            }

            let (content, marker) = line.split_at(60);
            ///////////////////////////////
            // [0] END OF HEADER
            //     --> done parsing
            ///////////////////////////////
            if marker.trim().eq("END OF HEADER") {
                break;
            }
            ///////////////////////////////
            // COMMENTS are stored: "as is"
            ///////////////////////////////
            if marker.trim().eq("COMMENT") {
                // --> storing might be useful
                comments.push(content.trim().to_string());
                continue;

            } else if marker.contains("ANTENNA: B.SIGHT XYZ") {
            } else if marker.contains("ANTENNA: ZERODIR XYZ") {
            } else if marker.contains("ANTENNA: PHASECENTER") {
            } else if marker.contains("CENTER OF MASS: XYZ") {
            } else if marker.contains("PRN / BIAS / RMS") {
            } else if marker.contains("TIME REF STATION") {

                ///////////////////////////////////////////////////////
                // Handled cases
                ///////////////////////////////////////////////////////
            } else if marker.contains("ANTEX VERSION / SYST") {
                let (vers, system) = content.split_at(8);
                let vers = vers.trim();
                version = Version::from_str(vers).or(Err(ParsingError::AntexVersion))?;

                if let Ok(constell) = Constellation::from_str(system.trim()) {
                    constellation = Some(constell)
                }

                rinex_type = Type::AntennaData;

            } else if marker.contains("PCV TYPE / REFANT") {
                let (pcv_str, rem) = content.split_at(20);
                let (rel_type, rem) = rem.split_at(20);
                let (ref_sn, _) = rem.split_at(20);
                if let Ok(mut pcv) = Pcv::from_str(pcv_str.trim()) {
                    if pcv.is_relative() {
                        // try to parse "Relative Type"
                        if !rel_type.trim().is_empty() {
                            pcv = pcv.with_relative_type(rel_type.trim());
                        }
                    }
                    antex = antex.with_pcv_type(pcv);
                }
                if !ref_sn.trim().is_empty() {
                    antex = antex.with_reference_antenna_sn(ref_sn.trim());
                }
            } else if marker.contains("TYPE / SERIAL NO") {
                let items: Vec<&str> = content.split_ascii_whitespace().collect();
                if items.len() == 2 {
                    // Receiver antenna information
                    // like standard RINEX
                    let (model, rem) = content.split_at(20);
                    let (sn, _) = rem.split_at(20);
                    if let Some(a) = &mut rcvr_antenna {
                        *a = a.with_model(model.trim()).with_serial_number(sn.trim());
                    } else {
                        rcvr_antenna = Some(
                            Antenna::default()
                                .with_model(model.trim())
                                .with_serial_number(sn.trim()),
                        );
                    }
                } else if items.len() == 4 {
                    // Space Vehicle antenna information
                    // ANTEX RINEX specific
                    let (model, rem) = content.split_at(10);
                    let (svnn, rem) = rem.split_at(10);
                    let (cospar, _) = rem.split_at(10);
                    if let Ok(sv) = SV::from_str(svnn.trim()) {
                        if let Some(a) = &mut sv_antenna {
                            *a = a
                                .with_sv(sv)
                                .with_model(model.trim())
                                .with_cospar(cospar.trim());
                        } else {
                            sv_antenna = Some(
                                SvAntenna::default()
                                    .with_sv(sv)
                                    .with_model(model.trim())
                                    .with_cospar(cospar.trim()),
                            );
                        }
                    }
                }

            ///////////////////////////////////////
            // ==> from now on
            // RINEX standard / shared attributes
            ///////////////////////////////////////
            } else if marker.contains("RINEX VERSION / TYPE") {
                let (vers, rem) = line.split_at(20);
                let (type_str, rem) = rem.split_at(20);
                let (constell_str, _) = rem.split_at(20);

                let type_str = type_str.trim();
                let constell_str = constell_str.trim();

                // File type identification
                if type_str == "O" && constell_str == "D" {
                    rinex_type = Type::DORIS;
                } else {
                    rinex_type = Type::from_str(type_str)?;
                }

                // Determine (file) Constellation
                //  1. NAV SPECIAL CASE
                //  2. OTHER
                match rinex_type {
                    Type::NavigationData => {
                        if type_str.contains("GLONASS") {
                            // old GLONASS NAV : no constellation field
                            constellation = Some(Constellation::Glonass);
                        } else if type_str.contains("GPS NAV DATA") {
                            constellation = Some(Constellation::GPS);
                        } else if type_str.contains("IRNSS NAV DATA") {
                            constellation = Some(Constellation::IRNSS);
                        } else if type_str.contains("GNSS NAV DATA") {
                            constellation = Some(Constellation::Mixed);
                        } else if type_str.eq("NAVIGATION DATA") {
                            if constell_str.is_empty() {
                                // old GPS NAVIGATION DATA
                                constellation = Some(Constellation::GPS);
                            } else {
                                // Modern NAVIGATION DATA
                                if let Ok(c) = Constellation::from_str(constell_str) {
                                    constellation = Some(c);
                                }
                            }
                        }
                    },
                    Type::MeteoData | Type::DORIS => {
                        // no constellation associated to them
                    },
                    _ => {
                        // any other
                        // regular files
                        if let Ok(c) = Constellation::from_str(constell_str) {
                            constellation = Some(c);
                        }
                    },
                }
                /*
                 * Parse version descriptor
                 */
                let vers = vers.trim();
                version = Version::from_str(vers).or(Err(ParsingError::VersionParsing))?;
            } else if marker.contains("PGM / RUN BY / DATE") {
                let (pgm, rem) = line.split_at(20);
                let pgm = pgm.trim();
                if pgm.len() > 0 {
                    program = Some(pgm.to_string());
                }

                let (runby, rem) = rem.split_at(20);

                let runby = runby.trim();
                if runby.len() > 0 {
                    run_by = Some(runby.to_string());
                }

                let date_str = rem.split_at(20).0.trim();
                if date_str.len() > 0 {
                    date = Some(date_str.to_string());
                }
            } else if marker.contains("MARKER NAME") {
                let name = content.split_at(20).0.trim();
                geodetic_marker = Some(GeodeticMarker::default().with_name(name));
            } else if marker.contains("MARKER NUMBER") {
                let number = content.split_at(20).0.trim();
                if let Some(ref mut marker) = geodetic_marker {
                    *marker = marker.with_number(number);
                }
            } else if marker.contains("MARKER TYPE") {
                let code = content.split_at(20).0.trim();
                if let Ok(mtype) = MarkerType::from_str(code) {
                    if let Some(ref mut marker) = geodetic_marker {
                        marker.marker_type = Some(mtype);
                    }
                }
            } else if marker.contains("OBSERVER / AGENCY") {
                let (obs, ag) = content.split_at(20);
                let obs = obs.trim();
                let ag = ag.trim();

                if obs.len() > 0 {
                    observer = Some(obs.to_string());
                }

                if ag.len() > 0 {
                    agency = Some(ag.to_string());
                }
            } else if marker.contains("REC # / TYPE / VERS") {
                if let Ok(receiver) = Receiver::from_str(content) {
                    rcvr = Some(receiver);
                }
            } else if marker.contains("SYS / PCVS APPLIED") {
                let (gnss, rem) = content.split_at(2);
                let (program, rem) = rem.split_at(18);
                let (url, _) = rem.split_at(40);

                let gnss = gnss.trim();
                let gnss = Constellation::from_str(gnss.trim())?;

                let pcv = PcvCompensation {
                    program: {
                        let program = program.trim();
                        if program.eq("") {
                            String::from("Unknown")
                        } else {
                            program.to_string()
                        }
                    },
                    constellation: gnss,
                    url: {
                        let url = url.trim();
                        if url.eq("") {
                            String::from("Unknown")
                        } else {
                            url.to_string()
                        }
                    },
                };

                pcv_compensations.push(pcv);
            } else if marker.contains("SYS / DCBS APPLIED") {
                let (gnss, rem) = content.split_at(2);
                let (program, rem) = rem.split_at(18);
                let (url, _) = rem.split_at(40);

                let gnss = gnss.trim();
                let gnss = Constellation::from_str(gnss.trim())?;

                let dcb = DcbCompensation {
                    program: {
                        let program = program.trim();
                        if program.eq("") {
                            String::from("Unknown")
                        } else {
                            program.to_string()
                        }
                    },
                    constellation: gnss,
                    url: {
                        let url = url.trim();
                        if url.eq("") {
                            String::from("Unknown")
                        } else {
                            url.to_string()
                        }
                    },
                };

                dcb_compensations.push(dcb);
            
            } else if marker.contains("MERGED FILE") {
                //TODO V > 3
                // nb# of merged files
            
            } else if marker.contains("STATION INFORMATION") {
                let url = content.split_at(40).0.trim(); //TODO confirm please
                if url.len() > 0 {
                    station_url = Some(url.to_string());
                }
            
            } else if marker.contains("LICENSE OF USE") {
                let lic = content.split_at(40).0.trim(); //TODO confirm please
                if lic.len() > 0 {
                    license = Some(lic.to_string());
                }
            } else if marker.contains("WAVELENGTH FACT L1/2") {
                //TODO
            
            } else if marker.contains("APPROX POSITION XYZ") {
                let mut num_items = 0;
                let (mut x_ecef_m, mut y_ecef_m, mut z_ecef_m) = (0.0_f64, 0.0_f64, 0.0_f64);

                for (nth, item) in content.split_ascii_whitespace().enumerate() {
                    if let Ok(ecef_m) = item.trim().parse::<f64>() {
                        match nth {
                            0 => {
                                x_ecef_m = ecef_m;
                            },
                            1 => {
                                y_ecef_m = ecef_m;
                            },
                            2 => {
                                num_items = 3;
                                z_ecef_m = ecef_m;
                            },
                            _ => {},
                        }
                    }
                }

                if num_items == 3 {
                    rx_position = Some((x_ecef_m, y_ecef_m, z_ecef_m));
                }
            } else if marker.contains("ANT # / TYPE") {
                let (sn, rem) = content.split_at(20);
                let (model, _) = rem.split_at(20);

                rcvr_antenna = Some(
                    Antenna::default()
                        .with_model(model.trim())
                        .with_serial_number(sn.trim()),
                );
            } else if marker.contains("ANTENNA: DELTA X/Y/Z") {
                // Antenna Base/Reference Coordinates
                let items: Vec<&str> = content.split_ascii_whitespace().collect();

                let x = items[0].trim();
                let x = f64::from_str(x).or(Err(ParsingError::AntennaCoordinates))?;

                let y = items[1].trim();
                let y = f64::from_str(y).or(Err(ParsingError::AntennaCoordinates))?;

                let z = items[2].trim();
                let z = f64::from_str(z).or(Err(ParsingError::AntennaCoordinates))?;

                if let Some(ant) = &mut rcvr_antenna {
                    *ant = ant.with_base_coordinates((x, y, z));
                } else {
                    rcvr_antenna = Some(Antenna::default().with_base_coordinates((x, y, z)));
                }
            } else if marker.contains("ANTENNA: DELTA H/E/N") {
                // Antenna H/E/N eccentricity components
                let (h, rem) = content.split_at(15);
                let (e, rem) = rem.split_at(15);
                let (n, _) = rem.split_at(15);
                if let Ok(h) = f64::from_str(h.trim()) {
                    if let Ok(e) = f64::from_str(e.trim()) {
                        if let Ok(n) = f64::from_str(n.trim()) {
                            if let Some(a) = &mut rcvr_antenna {
                                *a = a
                                    .with_height(h)
                                    .with_eastern_component(e)
                                    .with_northern_component(n);
                            } else {
                                rcvr_antenna = Some(
                                    Antenna::default()
                                        .with_height(h)
                                        .with_eastern_component(e)
                                        .with_northern_component(n),
                                );
                            }
                        }
                    }
                }
            
            } else if marker.contains("STATION NAME / NUM") {
                let (name, domes) = content.split_at(4);
                clock = clock.site(name.trim());
                if let Ok(domes) = DOMES::from_str(domes.trim()) {
                    clock = clock.domes(domes);
                }
            } else if marker.contains("SIGNAL STRENGHT UNIT") {
                //TODO
            } else if marker.contains("INTERVAL") {
                let intv_str = content.split_at(20).0.trim();
                if let Ok(interval) = f64::from_str(intv_str) {
                    if interval > 0.0 {
                        // INTERVAL = '0' may exist, in case
                        // of Varying TEC map intervals
                        sampling_interval = Some(Duration::from_seconds(interval));
                    }
                }
            } else if marker.contains("COSPAR NUMBER") {
                cospar = Some(COSPAR::from_str(content.trim())?);
            } else if marker.contains("GLONASS SLOT / FRQ #") {
                //TODO
                // This should be used when dealing with Glonass carriers

                let slots = content.split_at(4).1.trim();
                for i in 0..num_integer::div_ceil(slots.len(), 7) {
                    let svnn = &slots[i * 7..i * 7 + 4];
                    let chx = &slots[i * 7 + 4..std::cmp::min(i * 7 + 4 + 3, slots.len())];
                    if let Ok(svnn) = SV::from_str(svnn.trim()) {
                        if let Ok(chx) = chx.trim().parse::<i8>() {
                            glo_channels.insert(svnn, chx);
                        }
                    }
                }
            }
        }

        Ok(Header {
            version,
            rinex_type,
            constellation,
            comments,
            program,
            run_by,
            date,
            geodetic_marker,
            agency,
            observer,
            license,
            doi,
            station_url,
            rcvr,
            cospar,
            glo_channels,
            leap,
            rx_position,
            ionod_corrections,
            dcb_compensations,
            pcv_compensations,
            wavelengths: None,
            sampling_interval,
            rcvr_antenna,
            sv_antenna,
            // RINEX specific
            obs: {
                if rinex_type == Type::ObservationData {
                    Some(observation)
                } else {
                    None
                }
            },
            nav: {
                if rinex_type == Type::NavigationData {
                    Some(nav)
                } else {
                    None
                }
            },
            meteo: {
                if rinex_type == Type::MeteoData {
                    Some(meteo)
                } else {
                    None
                }
            },
            clock: {
                if rinex_type == Type::ClockData {
                    Some(clock)
                } else {
                    None
                }
            },
            antex: {
                if rinex_type == Type::AntennaData {
                    Some(antex)
                } else {
                    None
                }
            },
            doris: {
                if rinex_type == Type::DORIS {
                    Some(doris)
                } else {
                    None
                }
            },
        })
    }

    fn parse_time_of_obs(content: &str) -> Result<Epoch, ParsingError> {
        let (_, rem) = content.split_at(2);
        let (y, rem) = rem.split_at(4);
        let (m, rem) = rem.split_at(6);
        let (d, rem) = rem.split_at(6);
        let (hh, rem) = rem.split_at(6);
        let (mm, rem) = rem.split_at(6);
        let (ss, rem) = rem.split_at(5);
        let (_dot, rem) = rem.split_at(1);
        let (ns, rem) = rem.split_at(8);

        // println!("Y \"{}\" M \"{}\" D \"{}\" HH \"{}\" MM \"{}\" SS \"{}\" NS \"{}\"", y, m, d, hh, mm, ss, ns); // DEBUG
        let mut y = y
            .trim()
            .parse::<u32>()
            .map_err(|_| ParsingError::DatetimeParsing)?;

        // handle OLD RINEX problem
        if y >= 79 && y <= 99 {
            y += 1900;
        } else if y < 79 {
            y += 2000;
        }

        let m = m
            .trim()
            .parse::<u8>()
            .map_err(|_| ParsingError::DatetimeParsing)?;

        let d = d
            .trim()
            .parse::<u8>()
            .map_err(|_| ParsingError::DatetimeParsing)?;

        let hh = hh
            .trim()
            .parse::<u8>()
            .map_err(|_| ParsingError::DatetimeParsing)?;

        let mm = mm
            .trim()
            .parse::<u8>()
            .map_err(|_| ParsingError::DatetimeParsing)?;

        let ss = ss
            .trim()
            .parse::<u8>()
            .map_err(|_| ParsingError::DatetimeParsing)?;

        let ns = ns
            .trim()
            .parse::<u32>()
            .map_err(|_| ParsingError::DatetimeParsing)?;

        /*
         * We set TAI as "default" Timescale.
         * Timescale might be omitted in Old RINEX formats,
         * In this case, we exit with "TAI" and handle that externally.
         */
        let mut ts = TimeScale::TAI;
        let rem = rem.trim();

        /*
         * Handles DORIS measurement special case,
         * offset from TAI, that we will convert back to TAI later
         */
        if !rem.is_empty() && rem != "DOR" {
            ts = TimeScale::from_str(rem.trim())?;
        }

        Epoch::from_str(&format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:08} {}",
            y, m, d, hh, mm, ss, ns, ts
        ))
        .map_err(|_| ParsingError::DatetimeParsing)
    }

    /*
     * Parse IONEX grid
     */
    fn parse_grid(line: &str) -> Result<Linspace, ParsingError> {
        let mut start = 0.0_f64;
        let mut end = 0.0_f64;
        let mut spacing = 0.0_f64;
        for (index, item) in line.split_ascii_whitespace().enumerate() {
            let item = item.trim();
            match index {
                0 => {
                    start = f64::from_str(item).or(Err(ParsingError::IonexGridSpecs))?;
                },
                1 => {
                    end = f64::from_str(item).or(Err(ParsingError::IonexGridSpecs))?;
                },
                2 => {
                    spacing = f64::from_str(item).or(Err(ParsingError::IonexGridSpecs))?;
                },
                _ => {},
            }
        }
        if spacing == 0.0 {
            // avoid linspace verification in this case
            Ok(Linspace {
                start,
                end,
                spacing,
            })
        } else {
            let grid = Linspace::new(start, end, spacing)?;
            Ok(grid)
        }
    }

    /// Parse list of [Observable]s which applies to both METEO and OBS RINEX
    pub(crate) fn parse_v2_observables(
        line: &str,
        constell: Option<Constellation>,
        meteo: &mut MeteoHeader,
        observation: &mut ObservationHeader,
    ) {
        lazy_static! {
            /*
             *  We support GPS, Glonass, Galileo, SBAS and BDS as per v2.11.
             */
            static ref KNOWN_V2_CONSTELLS: [Constellation; 5] = [
                Constellation::GPS,
                Constellation::SBAS,
                Constellation::Glonass,
                Constellation::Galileo,
                Constellation::BeiDou,
            ];
        }
        let line = line.split_at(6).1;
        for item in line.split_ascii_whitespace() {
            if let Ok(obs) = Observable::from_str(item.trim()) {
                match constell {
                    Some(Constellation::Mixed) => {
                        for constell in KNOWN_V2_CONSTELLS.iter() {
                            if let Some(codes) = observation.codes.get_mut(constell) {
                                codes.push(obs.clone());
                            } else {
                                observation.codes.insert(*constell, vec![obs.clone()]);
                            }
                        }
                    },
                    Some(c) => {
                        if let Some(codes) = observation.codes.get_mut(&c) {
                            codes.push(obs.clone());
                        } else {
                            observation.codes.insert(c, vec![obs.clone()]);
                        }
                    },
                    None => meteo.codes.push(obs),
                }
            }
        }
    }

    /// Parse list of [Observable]s which applies to both METEO and OBS RINEX
    fn parse_v3_observables(
        line: &str,
        current_constell: &mut Option<Constellation>,
        observation: &mut ObservationHeader,
    ) {
        let (possible_counter, items) = line.split_at(6);
        if !possible_counter.is_empty() {
            let code = &possible_counter[..1];
            if let Ok(c) = Constellation::from_str(code) {
                *current_constell = Some(c);
            }
        }
        if let Some(constell) = current_constell {
            // system correctly identified
            for item in items.split_ascii_whitespace() {
                if let Ok(observable) = Observable::from_str(item) {
                    if let Some(codes) = observation.codes.get_mut(constell) {
                        codes.push(observable);
                    } else {
                        observation.codes.insert(*constell, vec![observable]);
                    }
                }
            }
        }
    }
    /*
     * Parse list of DORIS observables
     */
    fn parse_doris_observables(line: &str, doris: &mut DorisHeader) {
        let items = line.split_at(6).1;
        for item in items.split_ascii_whitespace() {
            if let Ok(observable) = Observable::from_str(item) {
                doris.observables.push(observable);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::prelude::{Epoch, Header};
    use std::str::FromStr;

    #[test]
    fn parse_time_of_obs() {
        let content = "  2021    12    21     0     0    0.0000000     GPS";
        let parsed = Header::parse_time_of_obs(&content).unwrap();
        assert_eq!(parsed, Epoch::from_str("2021-12-21T00:00:00 GPST").unwrap());

        let content = "  1995    01    01    00    00   00.000000             ";
        let parsed = Header::parse_time_of_obs(&content).unwrap();
        assert_eq!(parsed, Epoch::from_str("1995-01-01T00:00:00 TAI").unwrap());
    }
}
