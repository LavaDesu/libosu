use std::cmp::Ordering;
use std::collections::BTreeSet;

use num_rational::Ratio;

use SampleSet;

/// A struct representing a _precise_ location in time.
///
/// This enum represents a timestamp by either an absolute timestamp (milliseconds), or a tuple
/// (t, m, i, d) where _t_ is the `TimingPoint` that it's relative to, _m_ is the measure number
/// from within this timing section, _d_ is a value representing the meter (for example, 0 =
/// 1/1 meter, 1 = 1/2 meter, 3 = 1/4 meter, etc.), and _i_ is the index from the start of the measure.
#[derive(Debug)]
pub enum TimeLocation<'map> {
    /// Absolute timing in terms of number of milliseconds since the beginning of the audio file.
    /// Note that because this is an `i32`, the time is allowed to be negative.
    Absolute(i32),
    /// Relative timing based on an existing TimingPoint. The lifetime of this TimeLocation thus
    /// depends on the lifetime of the map.
    /// TODO: someday replace this with some kind of fraction class instead of this tuple hack
    Relative(&'map TimingPoint<'map>, u32, Ratio<u32>),
}

/// An enum distinguishing between inherited and uninherited timing points.
#[derive(Debug)]
pub enum TimingPointKind<'map> {
    /// Uninherited timing point
    Uninherited {
        /// BPM (beats per minute) of this timing section
        bpm: f64,
        /// The number of beats in a single measure
        meter: u32,
        /// List of inherited timing points that belong to this section.
        children: BTreeSet<TimingPoint<'map>>,
    },
    /// Inherited timing point
    Inherited {
        /// The uninherited timing point to which this timing point belongs.
        /// This field is an option because parsing and tree-building occur in different stages.
        parent: Option<&'map TimingPoint<'map>>,
        /// Slider velocity multiplier
        slider_velocity: f64,
    },
}

/// A timing point, which represents configuration settings for a timing section.
///
/// This is a generic timing point struct representing both inherited and uninherited timing
/// points, distinguished by the `kind` field.
#[derive(Debug)]
pub struct TimingPoint<'map> {
    /// The timestamp of this timing point, represented as a `TimeLocation`.
    pub time: TimeLocation<'map>,
    /// Whether or not Kiai time should be on for this timing point.
    pub kiai: bool,
    /// The sample set associated with this timing section.
    pub sample_set: SampleSet,
    /// Index (if using a custom sample)
    pub sample_index: u32,
    /// Volume of this timing section.
    pub volume: u16,
    /// The type of this timing point. See `TimingPointKind`.
    pub kind: TimingPointKind<'map>,
}

impl<'map> TimeLocation<'map> {
    /// Converts any `TimeLocation` into an absolute time in milliseconds from the beginning of the
    /// audio file.
    pub fn into_milliseconds(&self) -> i32 {
        match self {
            TimeLocation::Absolute(ref val) => *val,
            TimeLocation::Relative(ref tp, ref m, ref f) => {
                // the start of the previous timing point
                let base = tp.time.into_milliseconds();

                // milliseconds per beat
                let mpb = 60_000.0 / tp.get_bpm();

                // milliseconds per measure
                let mpm = mpb * (tp.get_meter() as f64);

                // amount of time from the timing point to the beginning of the current measure
                // this is equal to (milliseconds / measure) * (# measures)
                let measure_offset = mpm * (*m as f64);

                // this is the fractional part, from the beginning of the measure
                let remaining_offset = (*f.numer() as f64) * mpm / (*f.denom() as f64);

                // ok now just add it all together
                base + (measure_offset + remaining_offset) as i32
            }
        }
    }

    /// Converts any `TimeLocation` into a relative time tuple given a `TimingPoint`.
    pub fn approximate(&self, tp: &'map TimingPoint) -> (u32, Ratio<u32>) {
        match self {
            TimeLocation::Absolute(ref val) => {
                println!("approximating {} with {:?}", val, tp);
                // this is going to be black magic btw

                // in this function i'm going to assume that the osu editor is _extremely_
                // accurate, and all inaccuracies up to 2ms will be accommodated. this essentially
                // means if your timestamp doesn't fall on a beat exactly, and it's also _not_ 2ms
                // from any well-established snapping, it's probably going to fail horribly (a.k.a.
                // report large numbers for d)

                // oh well, let's give this a shot

                // first, let's calculate the measure offset
                // (using all the stuff from into_milliseconds above)
                let mpb = 60_000.0 / tp.get_bpm();
                let mpm = mpb * (tp.get_meter() as f64);
                let base = tp.time.into_milliseconds();
                let cur = *val;
                let measures = ((cur - base) as f64 / mpm) as i32;
                println!(
                    "cur: {}, base: {}, measures: {}, mpm: {}",
                    cur, base, measures, mpm
                );

                // approximate time that our measure starts
                let measure_start = base + (measures as f64 * mpm) as i32;
                let offset = cur - measure_start;
                println!("meausre_start: {}, offset: {}", measure_start, offset);

                // now, enumerate several well-established snappings
                let mut snappings = BTreeSet::new();
                for d in vec![1, 2, 3, 4, 6, 8, 12, 16] {
                    for i in 0..d {
                        let snap = (mpm * i as f64 / d as f64) as i32;
                        snappings.insert((i, d, snap));
                    }
                }
                println!("snappings {:?}", snappings);

                // now find out which one's the closest
                let mut distances = snappings
                    .into_iter()
                    .map(|(i, d, n)| (i, d, (offset - n).abs()))
                    .collect::<Vec<_>>();
                distances.sort_unstable_by(|(_, _, n1), (_, _, n2)| n1.cmp(n2));
                println!("distances {:?}", distances);

                // now see how accurate the first one is
                let (i, d, n) = distances.first().unwrap();
                if *n < 3 {
                    // yay accurate
                    return (measures as u32, Ratio::new(*i as u32, *d as u32));
                }

                // i'll worry about this later
                // this is probably going to just be some fraction approximation algorithm tho
                (0, Ratio::from(0))
            }
            TimeLocation::Relative(ref tp, _, _) => {
                // need to reconstruct the TimeLocation because we could be using a different
                // timing point
                // TODO: if the timing point is the same, return immediately
                TimeLocation::Absolute(self.into_milliseconds()).approximate(tp)
            }
        }
    }
}

impl<'map> Eq for TimeLocation<'map> {}

impl<'map> PartialEq for TimeLocation<'map> {
    fn eq(&self, other: &TimeLocation) -> bool {
        self.into_milliseconds() == other.into_milliseconds()
    }
}

impl<'map> Ord for TimeLocation<'map> {
    fn cmp(&self, other: &TimeLocation) -> Ordering {
        self.into_milliseconds().cmp(&other.into_milliseconds())
    }
}

impl<'map> PartialOrd for TimeLocation<'map> {
    fn partial_cmp(&self, other: &TimeLocation) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'map> TimingPoint<'map> {
    /// Gets the closest parent that is an uninherited timing point.
    pub fn get_uninherited_ancestor(&'map self) -> &'map TimingPoint<'map> {
        match &self.kind {
            &TimingPointKind::Uninherited { .. } => self,
            &TimingPointKind::Inherited { ref parent, .. } => match parent {
                Some(_parent) => _parent.get_uninherited_ancestor(),
                None => panic!("Inherited timing point does not have a parent."),
            },
        }
    }
    /// Gets the BPM of this timing section by climbing the timing section tree.
    pub fn get_bpm(&self) -> f64 {
        let ancestor = self.get_uninherited_ancestor();
        match &ancestor.kind {
            &TimingPointKind::Uninherited { ref bpm, .. } => *bpm,
            _ => panic!("The ancestor should always be an Uninherited timing point."),
        }
    }

    /// Gets the meter of this timing section by climbing the timing section tree.
    pub fn get_meter(&self) -> u32 {
        let ancestor = self.get_uninherited_ancestor();
        match &ancestor.kind {
            &TimingPointKind::Uninherited { ref meter, .. } => *meter,
            _ => panic!("The ancestor should always be an Uninherited timing point."),
        }
    }
}

impl<'map> Eq for TimingPoint<'map> {}

impl<'map> PartialEq for TimingPoint<'map> {
    fn eq(&self, other: &TimingPoint) -> bool {
        self.time.eq(&other.time)
    }
}

impl<'map> Ord for TimingPoint<'map> {
    fn cmp(&self, other: &TimingPoint) -> Ordering {
        self.time.cmp(&other.time)
    }
}

impl<'map> PartialOrd for TimingPoint<'map> {
    fn partial_cmp(&self, other: &TimingPoint) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

mod tests {
    extern crate lazy_static;

    #[allow(unused_imports)]
    #[allow(non_upper_case_globals)]
    use super::*;

    lazy_static! {
        static ref TP: TimingPoint<'static> = TimingPoint {
            kind: TimingPointKind::Uninherited {
                bpm: 200.0,
                meter: 4,
                children: BTreeSet::new(),
            },
            time: TimeLocation::Absolute(12345),
            sample_set: SampleSet::Auto,
            sample_index: 0,
            volume: 100,
            kiai: false,
        };
        static ref ITP: TimingPoint<'static> = TimingPoint {
            kind: TimingPointKind::Inherited {
                parent: Some(&TP),
                slider_velocity: 0.0,
            },
            time: TimeLocation::Relative(&TP, 1, Ratio::from(0)),
            sample_set: SampleSet::Auto,
            sample_index: 0,
            volume: 80,
            kiai: false,
        };
    }

    fn get_test_data<'a>() -> Vec<(TimeLocation<'a>, i32)> {
        let test_data = vec![
            // uninherited timing points
            (TimeLocation::Relative(&TP, 0, Ratio::new(0, 1)), 12345), // no change from the measure at all
            (TimeLocation::Relative(&TP, 1, Ratio::new(0, 1)), 13545), // +1 measure (measure is 300ms, times 4 beats)
            (TimeLocation::Relative(&TP, 0, Ratio::new(1, 4)), 12645), // a single beat
            (TimeLocation::Relative(&TP, 0, Ratio::new(1, 2)), 12945), // half of a measure
            (TimeLocation::Relative(&TP, 0, Ratio::new(3, 4)), 13245), // 3 quarter notes
            // ok, on to inherited
            (TimeLocation::Relative(&ITP, 0, Ratio::new(0, 1)), 13545), // no change from the measure at all
            (TimeLocation::Relative(&ITP, 1, Ratio::new(0, 1)), 14745), // +1 measure, same as above
            (TimeLocation::Relative(&ITP, 0, Ratio::new(1, 4)), 13845), // a single beat
            (TimeLocation::Relative(&ITP, 0, Ratio::new(1, 2)), 14145), // half of a measure
            (TimeLocation::Relative(&ITP, 0, Ratio::new(3, 4)), 14445), // 3 quarter notes
        ];
        return test_data;
    }

    #[test]
    fn test_into_milliseconds() {
        let test_data = get_test_data();
        for (time, abs) in test_data.iter() {
            assert_eq!(time.into_milliseconds(), *abs);
        }
    }

    #[test]
    fn test_approximate() {
        let test_data = get_test_data();
        for (time, abs) in test_data.iter() {
            let t = TimeLocation::Absolute(*abs);
            match time {
                TimeLocation::Relative(tp, m, f) => {
                    let (m2, f2) = t.approximate(&tp);
                    assert_eq!((*m, *f), (m2, f2));
                }
                _ => panic!("This should never happen."),
            }
        }
    }
}
