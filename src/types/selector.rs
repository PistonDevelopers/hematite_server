use std::collections::HashMap;
use std::num::{ParseFloatError, ParseIntError};
use std::str::FromStr;

use crate::util::{Join, Range};

use regex::Regex;

#[derive(Clone, Debug, PartialEq, Eq)]
/// A selector attribute, representing either a specific value of type `T`, any value other than a specific one, or any value.
pub enum Attr<T> {
    Is(T),
    Not(T),
    Unspecified,
}

impl<T: From<String>> From<String> for Attr<T> {
    fn from(s: String) -> Attr<T> {
        if s.is_empty() {
            Attr::Unspecified
        } else if &s[..1] == "!" {
            Attr::Not(T::from(s[1..].to_string()))
        } else {
            Attr::Is(T::from(s))
        }
    }
}

impl<'a, T: From<String>> From<&'a str> for Attr<T> {
    fn from(s: &str) -> Attr<T> {
        if s.is_empty() {
            Attr::Unspecified
        } else if &s[..1] == "!" {
            Attr::Not(T::from(s[1..].to_string()))
        } else {
            Attr::Is(T::from(s.to_string()))
        }
    }
}

#[derive(Debug)]
pub enum Error {
    InvalidArgName(String),
    InvalidSigil(String),
    MalformedFloat(ParseFloatError),
    MalformedInt(ParseIntError),
    MalformedSelector,
    PositionalAfterNamed,
    TooManyPositionalArgs,
}

impl From<ParseFloatError> for Error {
    fn from(err: ParseFloatError) -> Error {
        Error::MalformedFloat(err)
    }
}

impl From<ParseIntError> for Error {
    fn from(err: ParseIntError) -> Error {
        Error::MalformedInt(err)
    }
}

#[derive(Clone, Debug, PartialEq)]
/// An entity selector used in commands, for example `@p` or `@e[type=Creeper,c=2]`.
pub struct EntitySelector {
    random: bool,
    position: [Option<i32>; 3],
    delta_pos: [Option<i32>; 3],
    radius: Range<i32>,
    gamemode: Option<u8>,
    count: i32,
    xp_level: Range<i32>,
    scores: HashMap<String, Range<i32>>,
    team: Attr<String>,
    name: Attr<String>,
    pitch: Range<f32>,
    yaw: Range<f32>,
    entity_type: Attr<String>,
}

impl EntitySelector {
    /// Returns a selector equivalent to `@a`, matching all players.
    #[must_use]
    pub fn all() -> EntitySelector {
        EntitySelector {
            entity_type: Attr::Is("Player".to_string()),
            ..EntitySelector::default()
        }
    }

    /// Returns a selector equivalent to `@p`, matching the nearest player.
    #[must_use]
    pub fn player() -> EntitySelector {
        EntitySelector {
            count: 1,
            entity_type: Attr::Is("Player".to_string()),
            ..EntitySelector::default()
        }
    }

    /// Returns a selector equivalent to `@r`, matching a random player.
    #[must_use]
    pub fn random() -> EntitySelector {
        EntitySelector {
            random: true,
            count: 1,
            entity_type: Attr::Is("Player".to_string()),
            ..EntitySelector::default()
        }
    }
}

impl Default for EntitySelector {
    /// Returns a selector equivalent to `@e`, matching all entities.
    fn default() -> EntitySelector {
        EntitySelector {
            random: false,
            position: [None, None, None],
            delta_pos: [None, None, None],
            radius: Range::from(..),
            gamemode: None,
            count: 0,
            xp_level: Range::from(..),
            scores: HashMap::new(),
            team: Attr::Unspecified,
            name: Attr::Unspecified,
            pitch: Range::from(..),
            yaw: Range::from(..),
            entity_type: Attr::Unspecified,
        }
    }
}

impl FromStr for EntitySelector {
    type Err = Error;

    fn from_str(s: &str) -> Result<EntitySelector, Error> {
        if let Some(captures) = Regex::new(r"^@(.)(\[(.*)])?$").unwrap().captures(s) {
            let mut result = match captures.at(1).unwrap() {
                "a" => EntitySelector::all(),
                "e" => EntitySelector::default(),
                "p" => EntitySelector::player(),
                "r" => EntitySelector::random(),
                sigil => return Err(Error::InvalidSigil(sigil.to_string())),
            };
            if let Some(args) = captures.at(3) {
                let mut positional_seen = 0_u8; // number of positional arguments (x, y, z, r) encountered
                let mut named_seen = false; // whether a named argument has been encountered
                for arg in args.split(',') {
                    if let Some(captures) = Regex::new("^(.*)=(.*)$").unwrap().captures(arg) {
                        // named argument
                        let key = captures.at(1).unwrap();
                        let value = captures.at(2).unwrap();
                        match key {
                            "x" => {
                                result.position[0] = Some(i32::from_str(value)?);
                            }
                            "y" => {
                                result.position[1] = Some(i32::from_str(value)?);
                            }
                            "z" => {
                                result.position[2] = Some(i32::from_str(value)?);
                            }
                            "dx" => {
                                result.delta_pos[0] = Some(i32::from_str(value)?);
                            }
                            "dy" => {
                                result.delta_pos[1] = Some(i32::from_str(value)?);
                            }
                            "dz" => {
                                result.delta_pos[2] = Some(i32::from_str(value)?);
                            }
                            "r" => {
                                result.radius.end = Some(i32::from_str(value)?);
                            }
                            "rm" => {
                                result.radius.start = Some(i32::from_str(value)?);
                            }
                            "m" => {
                                result.gamemode = Some(u8::from_str(value)?);
                            }
                            "c" => {
                                result.count = i32::from_str(value)?;
                            }
                            "l" => {
                                result.xp_level.end = Some(i32::from_str(value)?);
                            }
                            "lm" => {
                                result.xp_level.start = Some(i32::from_str(value)?);
                            }
                            "team" => result.team = Attr::from(value),
                            "name" => result.name = Attr::from(value),
                            "rx" => {
                                result.pitch.end = Some(f32::from_str(value)?);
                            }
                            "rxm" => {
                                result.pitch.start = Some(f32::from_str(value)?);
                            }
                            "ry" => {
                                result.yaw.end = Some(f32::from_str(value)?);
                            }
                            "rym" => {
                                result.yaw.start = Some(f32::from_str(value)?);
                            }
                            "type" => result.entity_type = Attr::from(value),
                            k => {
                                if let Some(captures) =
                                    Regex::new("score_([A-Za-z]+)").unwrap().captures(k)
                                {
                                    let objective = captures.at(1).unwrap();
                                    result
                                        .scores
                                        .entry(objective.to_string())
                                        .or_insert_with(|| Range::from(..))
                                        .end = Some(i32::from_str(value)?);
                                } else if let Some(captures) =
                                    Regex::new("score_([A-Za-z]+)_min").unwrap().captures(k)
                                {
                                    let objective = captures.at(1).unwrap();
                                    result
                                        .scores
                                        .entry(objective.to_string())
                                        .or_insert_with(|| Range::from(..))
                                        .start = Some(i32::from_str(value)?);
                                } else {
                                    return Err(Error::InvalidArgName(k.to_string()));
                                }
                            }
                        }
                        named_seen = true;
                    } else {
                        // positional argument
                        if named_seen {
                            return Err(Error::PositionalAfterNamed);
                        }
                        if Regex::new("^ *$").unwrap().is_match(arg) {
                            // empty, keep default
                        } else {
                            match positional_seen {
                                0 => {
                                    result.position[0] = Some(i32::from_str(arg)?);
                                }
                                1 => {
                                    result.position[1] = Some(i32::from_str(arg)?);
                                }
                                2 => {
                                    result.position[2] = Some(i32::from_str(arg)?);
                                }
                                3 => {
                                    result.radius = Range::from(..i32::from_str(arg)?);
                                }
                                _ => return Err(Error::TooManyPositionalArgs),
                            }
                        }
                        positional_seen += 1;
                    }
                }
            }
            Ok(result)
        } else {
            Err(Error::MalformedSelector)
        }
    }
}

macro_rules! push_args {
    ($args:ident, $($key:ident => $value:expr),*) => {{
        $(
            if let Some($key) = $value {
                $args.push(format!("{}={}", stringify!($key), $key));
            }
        )*
    }}
}

macro_rules! push_attrs {
    ($args:ident, $($key:expr => $value:expr),*) => {{
        $(
            match $value {
                Attr::Is(ref v) => { $args.push(format!("{}={}", $key, v)) }
                Attr::Not(ref v) => { $args.push(format!("{}=!{}", $key, v)) }
                Attr::Unspecified => ()
            }
        )*
    }}
}

impl<'a> From<&'a EntitySelector> for String {
    fn from(sel: &EntitySelector) -> String {
        let mut sigil = if sel.random { 'r' } else { 'e' };
        let mut args = vec![];
        // arguments that can be displayed as positional
        if let Some(x) = sel.position[0] {
            args.push(format!("{}", x));
            if let Some(y) = sel.position[1] {
                args.push(format!("{}", y));
                if let Some(z) = sel.position[2] {
                    args.push(format!("{}", z));
                    if let Some(r) = sel.radius.end {
                        args.push(format!("{}", r));
                    }
                } else {
                    push_args!(args, r => sel.radius.end);
                }
            } else {
                push_args!(args,
                    z => sel.position[2],
                    r => sel.radius.end
                );
            }
        } else {
            push_args!(args,
                y => sel.position[1],
                z => sel.position[2],
                r => sel.radius.end
            );
        }
        // named-only args
        push_args!(args,
            rm => sel.radius.start,
            dx => sel.delta_pos[0],
            dy => sel.delta_pos[1],
            dz => sel.delta_pos[2],
            m => sel.gamemode,
            l => sel.xp_level.end,
            lm => sel.xp_level.start,
            rx => sel.pitch.end,
            rxm => sel.pitch.start,
            ry => sel.yaw.end,
            rym => sel.yaw.start
        );
        push_attrs!(args,
            "team" => sel.team,
            "name" => sel.name
        );
        if !sel.random && sel.entity_type == Attr::Is("Player".to_string()) {
            // use @a or @p instead of annotating with type=Player
            sigil = if sel.count == 0 { 'a' } else { 'p' };
            if sel.count != 0 && sel.count != 1 {
                args.push(format!("c={}", sel.count));
            }
        } else {
            if sel.entity_type != Attr::Is("Player".to_string()) {
                // @r defaults to type=Player, so only include type if it's not Player
                push_attrs!(args, "type" => sel.entity_type);
            }
            if sel.count != if sel.random { 1 } else { 0 } {
                // default is c=1 for @r, c=0 for @e
                args.push(format!("c={}", sel.count));
            }
        }
        format!(
            "@{}{}",
            sigil,
            if args.is_empty() {
                "".to_string()
            } else {
                format!("[{}]", args.join(','))
            }
        )
    }
}

#[cfg(test)]
mod test {
    use crate::util::Range;
    use std::default::Default;
    use std::str::FromStr;

    use super::*;

    // Table driven tests
    struct TestCase<'a> {
        selector: EntitySelector,
        string: &'a str,
    }

    #[test]
    fn decode_selectors() {
        let test_cases = vec![TestCase {
            string: "@e[0,64,0,80,type=Creeper,c=-4]",
            selector: EntitySelector {
                position: [Some(0), Some(64), Some(0)],
                radius: Range::from(..80),
                entity_type: Attr::Is("Creeper".to_string()),
                count: -4,
                ..EntitySelector::default()
            },
        }];
        for TestCase { string, selector } in test_cases {
            assert_eq!(EntitySelector::from_str(string).unwrap(), selector);
        }
    }

    #[test]
    fn basic_selectors() {
        for sel in &["@a", "@e", "@p", "@r"] {
            assert_eq!(
                (*sel).to_string(),
                String::from(&EntitySelector::from_str(sel).unwrap())
            );
        }
    }
}
