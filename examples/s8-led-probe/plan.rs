//! Plan file: parsing, validation, wire bytes and the `--dump` text. Nothing here touches a device.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use anyhow::{Context as _, bail, ensure};
use serde::Deserialize;

pub const MAX_LOGICAL: u8 = 0x7f;
const WRAP_WIDTH: usize = 90;
const HEX_BYTES_PER_LINE: usize = 32;

#[derive(Clone, Copy, Debug)]
pub struct Report {
    pub id: u8,
    pub payload_len: usize,
}

pub const MIXER_REPORT_ID: u8 = 0x82;

const OUTPUT_REPORTS: [Report; 3] = [
    Report {
        id: 0x80,
        payload_len: 118,
    },
    Report {
        id: 0x81,
        payload_len: 118,
    },
    Report {
        id: MIXER_REPORT_ID,
        payload_len: 73,
    },
];

/// The only reports this tool may send: the two deck reports, plus the mixer
/// report when the caller passed `--allow-mixer`. There is no feature report.
pub fn allowed_reports(allow_mixer: bool) -> &'static [Report] {
    if allow_mixer {
        &OUTPUT_REPORTS
    } else {
        &OUTPUT_REPORTS[..2]
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPlan {
    #[serde(default)]
    frames: BTreeMap<String, RawFrame>,
    steps: Vec<RawStep>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFrame {
    report: String,
    set: Vec<RawSet>,
    #[serde(default)]
    above_logical_max: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStep {
    id: String,
    #[serde(default)]
    optional: bool,
    writes: Vec<RawWrite>,
    expect: String,
    ask: String,
    why: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawWrite {
    report: Option<String>,
    base: Option<String>,
    set: Vec<RawSet>,
    #[serde(default)]
    above_logical_max: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSet {
    offset: Option<usize>,
    from: Option<usize>,
    to: Option<usize>,
    value: String,
}

struct Frame {
    report: Report,
    wire: Vec<u8>,
    above_logical_max: bool,
}

pub struct Write {
    pub report: u8,
    pub wire: Vec<u8>,
    pub above_logical_max: bool,
}

pub struct Step {
    pub id: String,
    pub optional: bool,
    pub writes: Vec<Write>,
    pub expect: String,
    pub ask: String,
}

pub struct Plan {
    pub steps: Vec<Step>,
}

impl Plan {
    pub fn parse(text: &str, reports: &[Report]) -> anyhow::Result<Self> {
        let raw: RawPlan = serde_json::from_str(text).context("parse plan JSON")?;
        ensure!(!raw.steps.is_empty(), "plan has no steps");
        let mut frames = BTreeMap::new();
        for (name, frame) in &raw.frames {
            let build = || {
                let report = parse_report(&frame.report, reports)?;
                let wire = apply(report, None, &frame.set, frame.above_logical_max)?;
                anyhow::Ok(Frame {
                    report,
                    wire,
                    above_logical_max: frame.above_logical_max,
                })
            };
            frames.insert(
                name.clone(),
                build().with_context(|| format!("frame {name}"))?,
            );
        }
        let mut seen = BTreeSet::new();
        let mut steps = Vec::new();
        for step in raw.steps {
            ensure!(!step.id.is_empty(), "step with an empty id");
            ensure!(
                seen.insert(step.id.clone()),
                "duplicate step id {}",
                step.id
            );
            ensure!(!step.writes.is_empty(), "step {} has no writes", step.id);
            ensure!(
                !step.expect.is_empty() && !step.ask.is_empty() && !step.why.is_empty(),
                "step {} needs expect, ask and why",
                step.id
            );
            let writes = step
                .writes
                .iter()
                .enumerate()
                .map(|(index, write)| {
                    resolve_write(write, &frames, reports)
                        .with_context(|| format!("step {} write {}", step.id, index + 1))
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            steps.push(Step {
                id: step.id,
                optional: step.optional,
                writes,
                expect: step.expect,
                ask: step.ask,
            });
        }
        Ok(Self { steps })
    }

    pub fn step(&self, id: &str) -> Option<&Step> {
        self.steps.iter().find(|step| step.id == id)
    }

    /// Step ids in plan order; optional steps only when `all` is set.
    pub fn ids(&self, all: bool) -> Vec<&str> {
        self.steps
            .iter()
            .filter(|step| all || !step.optional)
            .map(|step| step.id.as_str())
            .collect()
    }
}

fn parse_byte(text: &str) -> anyhow::Result<u8> {
    let digits = text
        .strip_prefix("0x")
        .with_context(|| format!("{text:?} is not written as 0xNN"))?;
    u8::from_str_radix(digits, 16).with_context(|| format!("{text:?} is not a byte"))
}

fn parse_report(text: &str, reports: &[Report]) -> anyhow::Result<Report> {
    let id = parse_byte(text).context("report ID")?;
    if let Some(report) = reports.iter().find(|report| report.id == id) {
        return Ok(*report);
    }
    if id == MIXER_REPORT_ID {
        bail!("report ID 0x82 (mixer) is only accepted with --allow-mixer");
    }
    bail!(
        "report ID 0x{id:02x} is not allowed: this tool sends only 0x80 and 0x81 output reports, \
         and 0x82 with --allow-mixer; feature reports are not supported"
    );
}

/// Applies `sets` on top of `base` (or an all-zero payload) and returns the wire bytes.
fn apply(
    report: Report,
    base: Option<&[u8]>,
    sets: &[RawSet],
    above_logical_max: bool,
) -> anyhow::Result<Vec<u8>> {
    let limit = if above_logical_max {
        u8::MAX
    } else {
        MAX_LOGICAL
    };
    let mut wire = base.map_or_else(|| vec![0_u8; report.payload_len + 1], <[u8]>::to_vec);
    wire[0] = report.id;
    for set in sets {
        let value = parse_byte(&set.value).context("value")?;
        ensure!(
            value <= limit,
            "value 0x{value:02x} is above the logical maximum 0x7f and has no \
             above_logical_max marker"
        );
        let (from, to) = match (set.offset, set.from, set.to) {
            (Some(offset), None, None) => (offset, offset),
            (None, Some(from), Some(to)) => (from, to),
            _ => bail!("an assignment needs either offset or both from and to"),
        };
        ensure!(
            from <= to && to < report.payload_len,
            "offsets {from}..={to} are outside 0..={}",
            report.payload_len - 1
        );
        for byte in &mut wire[from + 1..=to + 1] {
            *byte = value;
        }
    }
    Ok(wire)
}

fn resolve_write(
    raw: &RawWrite,
    frames: &BTreeMap<String, Frame>,
    reports: &[Report],
) -> anyhow::Result<Write> {
    let base = match &raw.base {
        Some(name) => Some(
            frames
                .get(name)
                .with_context(|| format!("unknown base frame {name}"))?,
        ),
        None => None,
    };
    let report = match (&raw.report, base) {
        (Some(text), _) => parse_report(text, reports)?,
        (None, Some(frame)) => frame.report,
        (None, None) => bail!("a write needs a report or a base frame"),
    };
    if let Some(frame) = base {
        ensure!(
            frame.report.id == report.id,
            "write report 0x{:02x} differs from its base frame report 0x{:02x}",
            report.id,
            frame.report.id
        );
    }
    let above_logical_max =
        raw.above_logical_max || base.is_some_and(|frame| frame.above_logical_max);
    let wire = apply(
        report,
        base.map(|frame| frame.wire.as_slice()),
        &raw.set,
        above_logical_max,
    )?;
    Ok(Write {
        report: report.id,
        wire,
        above_logical_max,
    })
}

/// One all-zero report per allowed ID, in order.
pub fn clear_wires(reports: &[Report]) -> Vec<Vec<u8>> {
    reports
        .iter()
        .map(|report| {
            let mut wire = vec![0_u8; report.payload_len + 1];
            wire[0] = report.id;
            wire
        })
        .collect()
}

fn wrap(first: &str, rest: &str, text: &str) -> String {
    let mut out = String::new();
    let mut line = first.to_owned();
    let mut fresh = true;
    for word in text.split_whitespace() {
        if !fresh && line.len() + 1 + word.len() > WRAP_WIDTH {
            out.push_str(&line);
            out.push('\n');
            rest.clone_into(&mut line);
            fresh = true;
        }
        if !fresh {
            line.push(' ');
        }
        line.push_str(word);
        fresh = false;
    }
    out.push_str(&line);
    out.push('\n');
    out
}

fn assignments(payload: &[u8]) -> String {
    let mut runs: Vec<String> = Vec::new();
    let mut start = 0;
    while start < payload.len() {
        let value = payload[start];
        let mut end = start;
        while end + 1 < payload.len() && payload[end + 1] == value {
            end += 1;
        }
        if value != 0 {
            runs.push(if start == end {
                format!("{start}={value:02x}")
            } else {
                format!("{start}-{end}={value:02x}")
            });
        }
        start = end + 1;
    }
    if runs.is_empty() {
        "(none)".to_owned()
    } else {
        runs.join(" ")
    }
}

/// Deterministic plain-ASCII text, offsets decimal, values hex.
pub fn dump(plan: &Plan) -> anyhow::Result<String> {
    let mut out = String::new();
    for step in &plan.steps {
        out.push_str(&step.id);
        out.push('\n');
        if step.optional {
            out.push_str("  optional: true\n");
        }
        for (index, write) in step.writes.iter().enumerate() {
            writeln!(
                out,
                "  write {}: report 0x{:02x}, {} bytes",
                index + 1,
                write.report,
                write.wire.len()
            )?;
            if write.above_logical_max {
                out.push_str("    above_logical_max: true\n");
            }
            out.push_str(&wrap(
                "    set: ",
                "         ",
                &assignments(&write.wire[1..]),
            ));
            for (line, chunk) in write.wire.chunks(HEX_BYTES_PER_LINE).enumerate() {
                write!(out, "    {:04x}: ", line * HEX_BYTES_PER_LINE)?;
                for byte in chunk {
                    write!(out, "{byte:02x}")?;
                }
                out.push('\n');
            }
        }
        out.push_str(&wrap("  expect: ", "    ", &step.expect));
        out.push_str(&wrap("  ask: ", "    ", &step.ask));
        out.push('\n');
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMMITTED: &str = include_str!("../../docs/s8/led-check-plan.json");
    const MIXER_COMMITTED: &str = include_str!("../../docs/s8/led-check-mixer-plan.json");
    const DECK_REPORTS: [Report; 2] = [OUTPUT_REPORTS[0], OUTPUT_REPORTS[1]];

    fn one_write(report: &str, set: &str, marker: &str) -> String {
        format!(
            r#"{{"steps":[{{"id":"X","writes":[{{"report":"{report}","set":[{set}]{marker}}}],
            "expect":"e","ask":"a","why":"w"}}]}}"#
        )
    }

    #[test]
    fn a_single_offset_lands_one_byte_after_the_report_id() -> anyhow::Result<()> {
        let text = one_write("0x80", r#"{"offset":59,"value":"0x7F"}"#, "");
        let plan = Plan::parse(&text, &DECK_REPORTS)?;
        let wire = &plan.steps[0].writes[0].wire;
        assert_eq!(wire.len(), 119);
        assert_eq!(wire[0], 0x80);
        assert_eq!(wire[60], 0x7f);
        assert_eq!(wire.iter().filter(|byte| **byte != 0).count(), 2);
        Ok(())
    }

    #[test]
    fn ranges_are_inclusive() -> anyhow::Result<()> {
        let text = one_write(
            "0x81",
            r#"{"from":117,"to":117,"value":"0x01"},{"from":0,"to":1,"value":"0x02"}"#,
            "",
        );
        let plan = Plan::parse(&text, &DECK_REPORTS)?;
        let wire = &plan.steps[0].writes[0].wire;
        assert_eq!(
            (wire[0], wire[1], wire[2], wire[3], wire[118]),
            (0x81, 2, 2, 0, 1)
        );
        Ok(())
    }

    #[test]
    fn only_the_deck_reports_are_accepted() {
        for report in ["0x82", "0x00", "0xf4", "0x7f", "0x100", "80"] {
            let text = one_write(report, "", "");
            assert!(Plan::parse(&text, &DECK_REPORTS).is_err(), "{report}");
        }
    }

    #[test]
    fn values_above_the_logical_maximum_need_the_marker() -> anyhow::Result<()> {
        let set = r#"{"offset":0,"value":"0x80"}"#;
        assert!(Plan::parse(&one_write("0x80", set, ""), &DECK_REPORTS).is_err());
        let marked = one_write("0x80", set, r#","above_logical_max":true"#);
        let plan = Plan::parse(&marked, &DECK_REPORTS)?;
        assert_eq!(plan.steps[0].writes[0].wire[1], 0x80);
        Ok(())
    }

    #[test]
    fn offsets_outside_the_payload_are_rejected() {
        for set in [
            r#"{"offset":118,"value":"0x01"}"#,
            r#"{"from":5,"to":4,"value":"0x01"}"#,
            r#"{"from":5,"value":"0x01"}"#,
            r#"{"value":"0x01"}"#,
        ] {
            assert!(
                Plan::parse(&one_write("0x80", set, ""), &DECK_REPORTS).is_err(),
                "{set}"
            );
        }
    }

    #[test]
    fn duplicate_ids_and_empty_steps_are_rejected() {
        let step =
            r#"{"id":"X","writes":[{"report":"0x80","set":[]}],"expect":"e","ask":"a","why":"w"}"#;
        assert!(Plan::parse(&format!(r#"{{"steps":[{step},{step}]}}"#), &DECK_REPORTS).is_err());
        let empty = r#"{"steps":[{"id":"X","writes":[],"expect":"e","ask":"a","why":"w"}]}"#;
        assert!(Plan::parse(empty, &DECK_REPORTS).is_err());
        assert!(Plan::parse(r#"{"steps":[]}"#, &DECK_REPORTS).is_err());
    }

    #[test]
    fn clear_wires_are_all_zero_after_the_id() {
        let wires = clear_wires(&DECK_REPORTS);
        assert_eq!(wires.len(), 2);
        for (wire, id) in wires.iter().zip([0x80, 0x81]) {
            assert_eq!(wire.len(), 119);
            assert_eq!(wire[0], id);
            assert!(wire[1..].iter().all(|byte| *byte == 0));
        }
    }

    #[test]
    fn the_committed_plan_validates_and_has_the_expected_steps() -> anyhow::Result<()> {
        let plan = Plan::parse(COMMITTED, &DECK_REPORTS)?;
        let ids: Vec<&str> = plan.steps.iter().map(|step| step.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "T0", "T1", "T2", "T3", "T4", "T5", "T6", "T7a", "T7b", "T7c", "T7d", "T7e", "T7f",
                "T7g", "T7h", "T8a", "T8b", "T8c", "T9a", "T9b", "T10a", "T10b", "T10c", "T11a",
                "T11b", "T12", "T13"
            ]
        );
        for step in &plan.steps {
            assert!(!step.ask.is_empty() && !step.expect.is_empty());
            for write in &step.writes {
                assert_eq!(write.wire.len(), 119);
            }
        }
        let by_id = |id: &str| plan.step(id).map(|step| &step.writes);
        let t0 = by_id("T0").context("T0")?;
        assert_eq!(t0.len(), 2);
        assert!(
            t0.iter()
                .all(|write| write.wire[1..].iter().all(|byte| *byte == 0))
        );
        let t1 = by_id("T1").context("T1")?;
        assert_eq!((t1[0].wire[0], t1[0].wire[60]), (0x80, 0x7f));
        let t9b = by_id("T9b").context("T9b")?;
        assert!(t9b[0].above_logical_max);
        assert_eq!(t9b[0].wire[1], 0xff);
        let t13 = by_id("T13").context("T13")?;
        assert_eq!(t13.len(), 2);
        assert_eq!(
            (t13[0].wire[60], t13[1].wire[60], t13[1].wire[53]),
            (0x7f, 0, 0x7f)
        );
        Ok(())
    }

    #[test]
    fn the_dump_is_ascii_with_short_lines_and_names_every_step() -> anyhow::Result<()> {
        let plan = Plan::parse(COMMITTED, &DECK_REPORTS)?;
        let text = dump(&plan)?;
        assert!(text.is_ascii());
        for line in text.lines() {
            assert!(line.len() < 95, "{line}");
        }
        for step in &plan.steps {
            assert!(text.lines().any(|line| line == step.id), "{}", step.id);
        }
        assert!(text.contains("    set: 59=7f\n"));
        assert!(text.contains("report 0x80, 119 bytes"));
        assert_eq!(text, dump(&plan)?);
        Ok(())
    }

    #[test]
    fn the_mixer_report_needs_the_flag_and_has_73_payload_bytes() -> anyhow::Result<()> {
        let text = one_write("0x82", r#"{"offset":72,"value":"0x7F"}"#, "");
        let error = Plan::parse(&text, allowed_reports(false))
            .err()
            .context("0x82 without the flag must fail")?;
        assert!(format!("{error:#}").contains("--allow-mixer"));
        let plan = Plan::parse(&text, allowed_reports(true))?;
        let wire = &plan.steps[0].writes[0].wire;
        assert_eq!((wire.len(), wire[0], wire[73]), (74, 0x82, 0x7f));
        let past = one_write("0x82", r#"{"offset":73,"value":"0x7F"}"#, "");
        assert!(Plan::parse(&past, allowed_reports(true)).is_err());
        assert!(Plan::parse(&text, allowed_reports(true)).is_ok());
        let clears = clear_wires(allowed_reports(true));
        assert_eq!(
            clears.iter().map(Vec::len).collect::<Vec<_>>(),
            [119, 119, 74]
        );
        assert_eq!(clear_wires(allowed_reports(false)).len(), 2);
        Ok(())
    }

    #[test]
    fn feature_report_ids_are_never_accepted() {
        for id in ["0xf3", "0xf4", "0xf8"] {
            assert!(
                Plan::parse(&one_write(id, "", ""), allowed_reports(true)).is_err(),
                "{id}"
            );
        }
    }

    #[test]
    fn a_write_is_applied_on_top_of_its_base_frame() -> anyhow::Result<()> {
        let text = r#"{"frames":{"B":{"report":"0x80","set":[{"from":1,"to":3,"value":"0x14"}]}},
            "steps":[{"id":"X","optional":true,"writes":[
            {"base":"B","set":[{"offset":2,"value":"0x7F"}]},
            {"base":"B","set":[]}],"expect":"e","ask":"a","why":"w"}]}"#;
        let plan = Plan::parse(text, &DECK_REPORTS)?;
        let step = &plan.steps[0];
        assert!(step.optional);
        assert_eq!(step.writes[0].wire[..5], [0x80, 0, 0x14, 0x7f, 0x14]);
        assert_eq!(step.writes[1].wire[..5], [0x80, 0, 0x14, 0x14, 0x14]);
        assert_eq!(plan.ids(false), Vec::<&str>::new());
        assert_eq!(plan.ids(true), ["X"]);
        for bad in [
            r#"{"base":"missing","set":[]}"#,
            r#"{"base":"B","report":"0x81","set":[]}"#,
            r#"{"set":[]}"#,
        ] {
            let broken = text.replace(r#"{"base":"B","set":[]}"#, bad);
            assert!(Plan::parse(&broken, &DECK_REPORTS).is_err(), "{bad}");
        }
        Ok(())
    }

    #[test]
    fn the_committed_mixer_plan_validates_with_the_flag_only() -> anyhow::Result<()> {
        assert!(Plan::parse(MIXER_COMMITTED, allowed_reports(false)).is_err());
        let plan = Plan::parse(MIXER_COMMITTED, allowed_reports(true))?;
        let m2 = plan.step("M2").context("M2")?;
        let wire = &m2.writes[0].wire;
        assert_eq!((wire.len(), wire[0]), (74, 0x82));
        assert_eq!(
            (wire[12], wire[46], wire[47], wire[49], wire[73]),
            (0x7f, 0x14, 0x14, 0x7f, 0x7f)
        );
        assert!(plan.ids(true).len() > plan.ids(false).len());
        assert_eq!(plan.ids(false).last().copied(), Some("M17"));
        assert!(dump(&plan)?.lines().all(|line| line.len() < 95));
        Ok(())
    }
}
