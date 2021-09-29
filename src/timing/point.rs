use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

use crate::errors::ParseError;
use crate::hitsounds::SampleSet;
use crate::timing::Millis;

/// Info for uninherited timing point
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct UninheritedTimingInfo {
    /// Milliseconds per beat (aka beat duration)
    pub mpb: f64,

    /// The number of beats in a single measure
    pub meter: u32,
}

/// Info for inherited timing point
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct InheritedTimingInfo {
    /// Slider velocity multiplier
    pub slider_velocity: f64,
}

/// An enum distinguishing between inherited and uninherited timing points.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TimingPointKind {
    /// Uninherited timing point
    Uninherited(UninheritedTimingInfo),

    /// Inherited timing point
    Inherited(InheritedTimingInfo),
}

/// A timing point, which represents configuration settings for a timing section.
///
/// This is a generic timing point struct representing both inherited and uninherited timing
/// points, distinguished by the `kind` field.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TimingPoint {
    /// The timestamp of this timing point, represented as a `TimeLocation`.
    pub time: Millis,

    /// Whether or not Kiai time should be on for this timing point.
    pub kiai: bool,

    /// The sample set associated with this timing section.
    pub sample_set: SampleSet,

    /// Index (if using a custom sample)
    pub sample_index: u32,

    /// Volume of this timing section.
    pub volume: u16,

    /// The type of this timing point. See `TimingPointKind`.
    pub kind: TimingPointKind,
}

impl Eq for TimingPoint {}

impl PartialEq for TimingPoint {
    fn eq(&self, other: &TimingPoint) -> bool {
        self.time.eq(&other.time)
    }
}

impl Ord for TimingPoint {
    fn cmp(&self, other: &TimingPoint) -> Ordering {
        self.time.cmp(&other.time)
    }
}

impl PartialOrd for TimingPoint {
    fn partial_cmp(&self, other: &TimingPoint) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl FromStr for TimingPoint {
    type Err = ParseError;

    fn from_str(input: &str) -> Result<TimingPoint, Self::Err> {
        let parts = input.split(',').collect::<Vec<_>>();

        let timestamp = parts[0].parse::<f64>()?;
        let mpb = parts[1].parse::<f64>()?;

        let mut meter = 4;
        if let Some(new_meter) = parts.get(2) {
            if !new_meter.is_empty() {
                meter = new_meter.parse::<u32>()?;
            }
        }

        let mut sample_set = 0;
        if let Some(new_sample_set) = parts.get(3) {
            if !new_sample_set.is_empty() {
                sample_set = new_sample_set.parse::<i32>()?;
            }
        }

        let mut sample_index = 0;
        if let Some(new_sample_index) = parts.get(4) {
            if !new_sample_index.is_empty() {
                sample_index = new_sample_index.parse::<u32>()?;
            }
        }

        // TODO: is the default supposed to be 0..?
        let mut volume = 0;
        if let Some(new_volume) = parts.get(5) {
            if !new_volume.is_empty() {
                volume = new_volume.parse::<u16>()?;
            }
        }

        let mut inherited = false;
        if let Some(new_inherited) = parts.get(6) {
            if !new_inherited.is_empty() {
                inherited = new_inherited.parse::<i32>()? == 0;
            }
        }

        let mut kiai = 4;
        if let Some(new_kiai) = parts.get(7) {
            if !new_kiai.is_empty() {
                kiai = new_kiai.parse::<i32>()?;
            }
        }

        // calculate bpm from mpb
        let _ = 60_000.0 / mpb;
        let time = Millis(timestamp.round() as i32);

        let timing_point = TimingPoint {
            kind: if inherited {
                TimingPointKind::Inherited(InheritedTimingInfo {
                    slider_velocity: -100.0 / mpb,
                })
            } else {
                TimingPointKind::Uninherited(UninheritedTimingInfo { mpb, meter })
            },
            kiai,
            sample_set: match sample_set {
                0 => SampleSet::None,
                1 => SampleSet::Normal,
                2 => SampleSet::Soft,
                3 => SampleSet::Drum,
                _ => panic!("Invalid sample set '{}'.", sample_set),
            },
            sample_index,
            volume,
            time,
        };

        Ok(timing_point)
    }
}

impl fmt::Display for TimingPoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let inherited = match self.kind {
            TimingPointKind::Inherited { .. } => 0,
            TimingPointKind::Uninherited { .. } => 1,
        };

        let (beat_length, meter) = match self.kind {
            TimingPointKind::Inherited(InheritedTimingInfo {
                slider_velocity, ..
            }) => (-100.0 / slider_velocity, 0),
            TimingPointKind::Uninherited(UninheritedTimingInfo { mpb, meter, .. }) => (mpb, meter),
        };

        write!(
            f,
            "{},{},{},{},{},{},{},{}",
            self.time.0,
            beat_length,
            meter,
            self.sample_set as i32,
            self.sample_index,
            self.volume,
            inherited,
            if self.kiai { 1 } else { 0 },
        )
    }
}
