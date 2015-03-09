use std::collections::{BTreeMap, BTreeSet};
use std::error::{Error, FromError};
use std::io;

use rustc_serialize::json;
use rustc_serialize::json::{Json, ToJson};

#[derive(Clone, Debug, PartialEq)]
pub enum ChatJsonError {
    MalformedJson(json::ParserError),
    IoError(io::Error),
    NotAnObject,
    InvalidFieldType,
    UnknownField(String),
    InvalidColor(String),
    InvalidClickEvent,
    InvalidHoverEvent
}

impl FromError<io::Error> for ChatJsonError {
    fn from_error(err: io::Error) -> ChatJsonError {
        ChatJsonError::IoError(err)
    }
}

impl FromError<json::ParserError> for ChatJsonError {
    fn from_error(err: json::ParserError) -> ChatJsonError {
        if let json::ParserError::IoError(e) = err {
            ChatJsonError::IoError(e)
        } else {
            ChatJsonError::MalformedJson(err)
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChatJson {
    pub msg: Message,
    pub extra: Option<Vec<Json>>,
    pub color: Option<Color>,
    pub formats: BTreeSet<Format>,
    pub click_event: Option<ClickEvent>,
    pub hover_event: Option<HoverEvent>,
    pub insertion: Option<String>
}

impl ChatJson {
    pub fn msg(msg: String) -> ChatJson {
        ChatJson {
            msg: Message::PlainText(msg), extra: None, color: None, formats: BTreeSet::new(),
            click_event: None, hover_event: None, insertion: None
        }
    }

    pub fn from_json(src: &mut io::Read) -> Result<ChatJson, ChatJsonError> {
        let json = try!(Json::from_reader(src));
        if let Json::Object(map) = json {
            let mut result = ChatJson::msg("".to_string());
            for (key, value) in map {
                println!("{:?}: {:?}", key, value);
                match key.as_slice() {
                    "text" => {
                        if let Json::String(string) = value {
                            result.msg = Message::PlainText(string);
                        } else {
                            return Err(ChatJsonError::InvalidFieldType);
                        }
                    },
                    "insertion" => {
                        if let Json::String(string) = value {
                            result.insertion = Some(string);
                        } else {
                            return Err(ChatJsonError::InvalidFieldType);
                        }
                    },
                    "color" => {
                        if let Json::String(string) = value {
                            result.color = match Color::from_string(&string) {
                                None => return Err(ChatJsonError::InvalidColor(string)),
                                c => c
                            };
                        } else {
                            return Err(ChatJsonError::InvalidFieldType);
                        }
                    },
                    // Handle all of the different format strings.
                    "bold"|"italic"|"underlined"|"strikethrough"|"obfuscated"|"reset"|"random" => {
                        if let Json::Boolean(b) = value {
                            if b == true {
                                result.formats.insert(Format::from_string(&key).unwrap());
                            }
                        } else {
                            return Err(ChatJsonError::InvalidFieldType);
                        }
                    },
                    // Handle the JSON format of click events.
                    "clickEvent" => {
                        if let Json::Object(event) = value {
                            // Get the `value` first.
                            let val: String = match event.get("value") {
                                Some(&Json::String(ref string)) => string.clone(),
                                _ => return Err(ChatJsonError::InvalidClickEvent)
                            };
                            // Handle the different click events.
                            if let Some(&Json::String(ref string)) = event.get("action") {
                                result.click_event = match string.as_slice() {
                                    "open_url" => Some(ClickEvent::OpenUrl(val)),
                                    "open_file" => Some(ClickEvent::OpenFile(val)),
                                    "run_command" => Some(ClickEvent::RunCommand(val)),
                                    "suggest_command" => Some(ClickEvent::SuggestCommand(val)),
                                    _ => return Err(ChatJsonError::InvalidClickEvent)
                                };
                            } else {
                                return Err(ChatJsonError::InvalidClickEvent);
                            }
                        }
                    },
                    // Handle the JSON format of hover events.
                    "hoverEvent" => {
                        if let Json::Object(event) = value {
                            // Get the `value` first.
                            let val: String = match event.get("value") {
                                Some(&Json::String(ref string)) => string.clone(),
                                _ => return Err(ChatJsonError::InvalidHoverEvent)
                            };
                            // Handle the different click events.
                            if let Some(&Json::String(ref string)) = event.get("action") {
                                result.hover_event = match string.as_slice() {
                                    "show_text" => Some(HoverEvent::Text(val)),
                                    "show_achievement" => Some(HoverEvent::Achievement(val)),
                                    "show_item" => Some(HoverEvent::Item(val)),
                                    _ => return Err(ChatJsonError::InvalidHoverEvent)
                                };
                            } else {
                                return Err(ChatJsonError::InvalidHoverEvent);
                            }
                        }
                    },
                    "extra" => {
                        if let Json::Array(extra) = value {
                            result.extra = Some(extra);
                        } else {
                            return Err(ChatJsonError::InvalidFieldType);
                        }
                    },
                    // TODO: Error on unknown key when the implementation is complete.
                    _ => (),
                };
            }
            Ok(result)
        } else {
            Err(ChatJsonError::NotAnObject)
        }
    }
}

impl ToJson for ChatJson {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();

        match self.msg {
            Message::PlainText(ref text) => {
                d.insert("text".to_string(), text.to_json());
            },
            _ => unimplemented!()
        };

        for format in self.formats.iter() {
            d.insert(format.to_string(), Json::Boolean(true));
        }

        if let Some(ref extra) = self.extra {
            d.insert("extra".to_string(), extra.to_json());
        }
        if let Some(ref color) = self.color {
            d.insert("color".to_string(), color.to_string().to_json());
        }
        if let Some(ref event) = self.click_event {
            d.insert("clickEvent".to_string(), event.to_json());
        }
        if let Some(ref event) = self.hover_event {
            d.insert("hoverEvent".to_string(), event.to_json());
        }
        if let Some(ref ins) = self.insertion {
            d.insert("insertion".to_string(), ins.to_json());
        }
        
        Json::Object(d)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Message {
    PlainText(String),
    Score(String, String),
    Translatable,
    Selector
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClickEvent {
    OpenUrl(String),
    OpenFile(String),
    RunCommand(String),
    SuggestCommand(String)
}

impl ToJson for ClickEvent {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        match self {
            &ClickEvent::OpenUrl(ref url) => {
                d.insert("action".to_string(), "open_url".to_json());
                d.insert("value".to_string(), url.to_json());
            },
            &ClickEvent::OpenFile(ref file) => {
                d.insert("action".to_string(), "open_file".to_json());
                d.insert("value".to_string(), file.to_json());
            },
            &ClickEvent::RunCommand(ref cmd) => {
                d.insert("action".to_string(), "run_command".to_json());
                d.insert("value".to_string(), cmd.to_json());
            },
            &ClickEvent::SuggestCommand(ref cmd) => {
                d.insert("action".to_string(), "suggest_command".to_json());
                d.insert("value".to_string(), cmd.to_json());
            }
        }
        Json::Object(d)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HoverEvent {
    Text(String),
    Achievement(String),
    Item(String)
}

impl ToJson for HoverEvent {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        match self {
            &HoverEvent::Text(ref text) => {
                d.insert("action".to_string(), "show_text".to_json());
                d.insert("value".to_string(), text.to_json());
            },
            &HoverEvent::Achievement(ref ach) => {
                d.insert("action".to_string(), "show_achievement".to_json());
                d.insert("value".to_string(), ach.to_json());
            },
            &HoverEvent::Item(ref item) => {
                d.insert("action".to_string(), "show_item".to_json());
                // The string is actually a JSON object, just in the form of a
                // string.
                d.insert("value".to_string(), item.to_json());
            }
        }
        Json::Object(d)
    }
}

#[derive(Copy, Debug, Clone, Eq, PartialEq)]
pub enum Color {
    Black       = 0x0,
    DarkBlue    = 0x1,
    DarkGreen   = 0x2,
    DarkCyan    = 0x3,
    DarkRed     = 0x4,
    Purple      = 0x5,
    Gold        = 0x6,
    Gray        = 0x7,
    DarkGray    = 0x8,
    Blue        = 0x9,
    BrightGreen = 0xa,
    Cyan        = 0xb,
    Red         = 0xc,
    Pink        = 0xd,
    Yellow      = 0xe,
    White       = 0xf
}

impl Color {
    pub fn to_string(&self) -> String {
        match self {
            &Color::Black => "black".to_string(),
            &Color::DarkBlue => "dark_blue".to_string(),
            &Color::DarkGreen => "dark_green".to_string(),
            &Color::DarkCyan => "dark_aqua".to_string(),
            &Color::DarkRed => "dark_red".to_string(),
            &Color::Purple => "dark_purple".to_string(),
            &Color::Gold => "gold".to_string(),
            &Color::Gray => "gray".to_string(),
            &Color::DarkGray => "dark_gray".to_string(),
            &Color::Blue => "blue".to_string(),
            &Color::BrightGreen => "green".to_string(),
            &Color::Cyan => "aqua".to_string(),
            &Color::Red => "red".to_string(),
            &Color::Pink => "light_purple".to_string(),
            &Color::Yellow => "yellow".to_string(),
            &Color::White => "white".to_string()
        }
    }

    pub fn from_string(string: &String) -> Option<Color> {
        match string.as_slice() {
            "black"        => Some(Color::Black),
            "dark_blue"    => Some(Color::DarkBlue),
            "dark_green"   => Some(Color::DarkGreen),
            "dark_aqua"    => Some(Color::DarkCyan),
            "dark_red"     => Some(Color::DarkRed),
            "dark_purple"  => Some(Color::Purple),
            "gold"         => Some(Color::Gold),
            "gray"         => Some(Color::Gray),
            "dark_gray"    => Some(Color::DarkGray),
            "blue"         => Some(Color::Blue),
            "green"        => Some(Color::BrightGreen),
            "aqua"         => Some(Color::Cyan),
            "red"          => Some(Color::Red),
            "light_purple" => Some(Color::Pink),
            "yellow"       => Some(Color::Yellow),
            "white"        => Some(Color::White),
            _              => None
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub enum Format {
    Bold, Underlined, Strikethrough, Italic, Obfuscated, Random, Reset
}

impl Format {
    pub fn to_string(&self) -> String {
        match self {
            &Format::Bold          => "bold".to_string(),
            &Format::Italic        => "italic".to_string(),
            &Format::Underlined    => "underlined".to_string(),
            &Format::Strikethrough => "strikethrough".to_string(),
            &Format::Obfuscated    => "obfuscated".to_string(),
            &Format::Random        => "random".to_string(),
            &Format::Reset         => "reset".to_string()
        }
    }

    pub fn from_string(string: &String) -> Option<Format> {
        match string.as_slice() {
            "bold"          => Some(Format::Bold),
            "italic"        => Some(Format::Italic),
            "underlined"    => Some(Format::Underlined),
            "strikethrough" => Some(Format::Strikethrough),
            "obfuscated"    => Some(Format::Obfuscated),
            "random"        => Some(Format::Random),
            "reset"         => Some(Format::Reset),
            _               => None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io;
    use rustc_serialize::json::{Builder, ToJson};

    #[test]
    fn chat_plain() {
        let msg = ChatJson::msg("Hello, world!".to_string());
        let blob = r#"{
            "text": "Hello, world!"
        }"#;
        let parsed = ChatJson::from_json(&mut io::Cursor::new(blob.as_bytes())).unwrap();
        assert_eq!(&msg, &parsed);
    }

    #[test]
    fn chat_invalid_field() {
        let blob = r#"{
            "text": true
        }"#;
        let parsed = ChatJson::from_json(&mut io::Cursor::new(blob.as_bytes()));
        assert_eq!(&parsed, &Err(ChatJsonError::InvalidFieldType));
    }

    #[test]
    fn chat_with_events() {
        let mut msg = ChatJson::msg("Hello, world!".to_string());
        msg.formats.insert(Format::Bold);
        msg.formats.insert(Format::Strikethrough);
        msg.color = Some(Color::Red);
        msg.insertion = Some("Hello, world!".to_string());
        msg.click_event = Some(ClickEvent::RunCommand("/time set day".to_string()));
        msg.hover_event = Some(HoverEvent::Text("Goodbye!".to_string()));

        let blob = r#"{
            "text": "Hello, world!",
            "bold": true,
            "strikethrough": true,
            "color":"red",
            "clickEvent":{
                "action":"run_command",
                "value": "/time set day"
            },
            "hoverEvent": {
                "action":"show_text",
                "value": "Goodbye!"
            },
            "insertion": "Hello, world!"
        }"#;

        let blob_json = Builder::new(blob.chars()).build().unwrap();
        assert_eq!(&blob_json, &msg.to_json());
        let parsed = ChatJson::from_json(&mut io::Cursor::new(blob.as_bytes())).unwrap();
        assert_eq!(&msg, &parsed);
    }

    #[test]
    fn chat_extra() {
        let blob = r#"{
            "text": "Hello world",
            "extra": [
                "Testing",
                {"translate":"demo.day.2"}
            ],
            "bold":true,
            "italic":false,
            "underlined": false,
            "strikethrough": true,
            "obfuscated": false,
            "color":"red",
            "clickEvent":{
                "action":"run_command",
                "value": "/time set day"
            },
            "hoverEvent": {
                "action":"show_text",
                "value": "Hello"
            },
            "insertion": "Hello world"
        }"#;

        let parsed = ChatJson::from_json(&mut io::Cursor::new(blob.as_bytes()));
        println!("{:?}", parsed);
    }
}
