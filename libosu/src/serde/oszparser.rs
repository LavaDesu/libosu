use failure::Error;
use regex::Regex;

use Beatmap;
use HitObject;
use HitObjectKind;
use Mode;
use Point;
use TimingPoint;

lazy_static! {
    static ref OSU_FORMAT_VERSION_RGX: Regex =
        Regex::new(r"^osu file format v(?P<version>\d+)$").unwrap();
    static ref SECTION_HEADER_RGX: Regex = Regex::new(r"^\[(?P<name>[A-Za-z]+)\]$").unwrap();
    static ref KEY_VALUE_RGX: Regex =
        Regex::new(r"^(?P<key>[A-Za-z0-9]+)\s*:\s*(?P<value>.+)$").unwrap();
}

pub trait OszParser<'src> {
    type Output;
    fn parse(input: &'src str) -> Result<Self::Output, Error>;
}

macro_rules! kvalue {
    ($captures:ident[$name:ident]: str) => {
        $name = String::from(&$captures["value"]);
    };
    ($captures:ident[$name:ident]: parse($type:ident)) => {
        $name = $captures["value"].parse::<$type>()?;
    };
}

impl<'map> OszParser<'map> for Beatmap<'map> {
    type Output = Beatmap<'map>;
    fn parse(input: &'map str) -> Result<Beatmap, Error> {
        let mut section = "Version".to_owned();
        let mut version = 0;

        let mut audio_filename = String::new();
        let mut audio_leadin = 0;
        let mut preview_time = 0;
        let mut countdown = 0;
        let mut stack_leniency = 0.0;
        let mut mode = 0;

        let mut hit_objects = Vec::new();
        let mut timing_points = Vec::new();

        for line in input.lines() {
            match SECTION_HEADER_RGX.captures(line) {
                Some(captures) => {
                    section = String::from(&captures["name"]);
                    continue;
                }
                None => (),
            }
            println!("\"{}\" {}", section, line);
            match section.as_ref() {
                "HitObjects" => {
                    let obj = HitObject::parse(line)?;
                    hit_objects.push(obj);
                }
                "Version" => {
                    if let Some(capture) = OSU_FORMAT_VERSION_RGX.captures(line) {
                        version = capture["version"].parse::<u32>()?;
                    }
                }
                _ => if let Some(captures) = KEY_VALUE_RGX.captures(line) {
                    match &captures["key"] {
                        "AudioFilename" => kvalue!(captures[audio_filename]: str),
                        "AudioLeadIn" => kvalue!(captures[audio_leadin]: parse(u32)),
                        "PreviewTime" => kvalue!(captures[preview_time]: parse(u32)),
                        "Countdown" => kvalue!(captures[countdown]: parse(u32)),
                        _ => (),
                    }
                },
            }
        }
        if version == 0 {
            bail!(
                "Could not find osu! file format version line. Check your beatmap and try again."
            );
        }
        Ok(Beatmap {
            version,
            audio_filename,
            audio_leadin,
            preview_time,
            countdown: countdown > 0,
            stack_leniency,
            mode: match mode {
                0 => Mode::Osu,
                1 => Mode::Taiko,
                2 => Mode::Catch,
                3 => Mode::Mania,
                _ => panic!("Invalid game mode."),
            },
            hit_objects,
            timing_points,
        })
    }
}

impl<'map> OszParser<'map> for HitObject {
    type Output = HitObject;
    fn parse(input: &'map str) -> Result<Self::Output, Error> {
        let parts = input.split(",");

        let obj = HitObject {
            kind: HitObjectKind::Circle,
            pos: Point(0, 0),
            start_time: 0,
        };
        Ok(obj)
    }
}

impl<'map> OszParser<'map> for TimingPoint<'map> {
    type Output = TimingPoint<'map>;
    fn parse(input: &'map str) -> Result<Self::Output, Error> {
        bail!("shiet");
    }
}
