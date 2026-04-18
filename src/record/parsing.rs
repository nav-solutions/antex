use crate::{
    antex::{
        record::{is_new_antenna, parse_antenna},
        Record as AntexRecord,
    },
    hatanaka::DecompressorExpert,
    is_rinex_comment,
    prelude::{Epoch, Header, ParsingError, TimeScale},
    record::{Comments, Record},
    types::Type,
};

use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Read},
    str::from_utf8,
};

#[cfg(feature = "log")]
use log::error;

impl Record {
    /// Parses [Record] section by consuming [Reader] entirely.
    /// This requires reference to [Header] that was just parsed by consuming [Reader] until this point.
    pub fn parse<R: Read>(
        header: &mut Header,
        reader: &mut BufReader<R>,
    ) -> Result<(Self, Comments), ParsingError> {
        // eos reached: process pending buffer & exit
        let mut eos = false;

        // current line storage
        let mut line_buf = String::with_capacity(128);

        // epoch storage
        let mut epoch_buf = String::with_capacity(1024);

        // comments management
        let mut comments: Comments = Comments::new();
        let mut comment_ts = Epoch::default();
        let mut comment_content = Vec::<String>::with_capacity(4);

        // Record
        let mut record = Record::new();

        // Iterate and consume, one line at a time
        while let Ok(size) = reader.read_line(&mut line_buf) {
            if size == 0 {
                // reached EOS
                // we might still have something to process prior exiting
                eos |= true;
            }

            // (special case) COMMENTS: store as is
            if is_rinex_comment(&line_buf) {
                let comment = line_buf.split_at(60).0.trim_end();
                comment_content.push(comment.to_string());

                // skip parsing
                line_buf.clear();
                continue;
            }

            // (special case) COMMENTS: store as is
            if line_buf.contains("COMMENT") {
                let content = line_buf.split_at(60).0.trim();
                if let Some(comments) = comments.get_mut(&comment_ts) {
                    comments.push(content.to_string());
                } else {
                    comments.insert(comment_ts, vec![content.to_string()]);
                }
                continue;
            }

            let mut new_antenna = false;

            // we're trying to stack a complete epoch
            // that we process once a new one appears
            if epoch_buf.len() > 0 {
                new_antenna = is_new_antenna(&line_buf, &header);

                // trick to force attempt on last iteration
                new_antenna |= eos;

                if new_antenna {
                    // new epoch appearing: process what we have buffered
                    // parsing method is format dependent
                    //println!("***MATCH***");
                    if let Ok((antenna, content)) = parse_antenna(&epoch_buf) {
                        record.push((antenna, content));
                    }
                }
            }

            // clear on new epoch detection
            if new_epoch {
                epoch_buf.clear();
            }

            // always stack new content
            epoch_buf.push_str(&line_buf);

            if eos || crinex_error {
                break;
            }

            line_buf.clear(); // always clear newline buf
        } //loop

        Ok((record, comments))
    }
}
