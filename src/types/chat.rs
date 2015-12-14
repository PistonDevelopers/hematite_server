use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::io;
use std::str::FromStr;

use rustc_serialize::{Encodable, Encoder};
use rustc_serialize::json::{self, Json, ToJson};

use packet::Protocol;
use types::EntitySelector;
use types::consts::Color;
use types::selector;

impl Protocol for ChatJson {
    type Clean = ChatJson;

    fn proto_len(value: &ChatJson) -> usize {
        <String as Protocol>::proto_len(&(value.to_json().to_string()))
    }

    fn proto_encode(value: &ChatJson, mut dst: &mut io::Write) -> io::Result<()> {
        let json_string = value.to_json().to_string();
        Ok(try!(<String as Protocol>::proto_encode(&json_string, dst)))
    }

    fn proto_decode(mut src: &mut io::Read) -> io::Result<ChatJson> {
        match ChatJson::from_reader(src) {
            Ok(chat_json) => Ok(chat_json),
            Err(err) => Err(io::Error::new(io::ErrorKind::Other, err))
        }
    }
}

#[derive(Debug)]
pub enum JsonType {
    Null,
    Boolean,
    Number,
    String,
    Array,
    Object
}

impl<'a> From<&'a Json> for JsonType {
    fn from(v: &Json) -> JsonType {
        match *v {
            Json::Null => JsonType::Null,
            Json::Boolean(_) => JsonType::Boolean,
            Json::I64(_) | Json::U64(_) | Json::F64(_) => JsonType::Number,
            Json::String(_) => JsonType::String,
            Json::Array(_) => JsonType::Array,
            Json::Object(_) => JsonType::Object
        }
    }
}

impl From<Json> for JsonType {
    fn from(v: Json) -> JsonType { JsonType::from(&v) }
}

#[derive(Debug)]
pub enum ChatJsonError {
    MalformedJson(json::ParserError),
    IoError(io::Error),
    InvalidFieldType { name: String, expected: JsonType, found: JsonType },
    InvalidRootType(JsonType),
    UnknownField(String),
    InvalidColor(String),
    InvalidClickEvent,
    InvalidHoverEvent,
    InvalidScore,
    SelectorError(selector::Error)
}

impl From<io::Error> for ChatJsonError {
    fn from(err: io::Error) -> ChatJsonError {
        ChatJsonError::IoError(err)
    }
}

impl From<json::ParserError> for ChatJsonError {
    fn from(err: json::ParserError) -> ChatJsonError {
        if let json::ParserError::IoError(e) = err {
            ChatJsonError::IoError(e)
        } else {
            ChatJsonError::MalformedJson(err)
        }
    }
}

impl From<selector::Error> for ChatJsonError {
    fn from(err: selector::Error) -> ChatJsonError {
        ChatJsonError::SelectorError(err)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChatJson {
    pub msg: Message,
    pub extra: Vec<ChatJson>,
    pub color: Option<Color>,
    pub formats: BTreeSet<Format>,
    pub click_event: Option<ClickEvent>,
    pub hover_event: Option<HoverEvent>,
    pub insertion: Option<String>
}

macro_rules! type_check {
    ($k:expr => $v:expr, $t:ident($p:pat) $b:block) => {{
        if let Json::$t($p) = $v $b else {
            return Err(ChatJsonError::InvalidFieldType {
                name: $k.to_string(),
                expected: JsonType::$t,
                found: JsonType::from($v)
            });
        }
    }}
}

impl ChatJson {
    pub fn from_reader(src: &mut io::Read) -> Result<ChatJson, ChatJsonError> {
        let json = try!(Json::from_reader(src));
        ChatJson::from_json(json)
    }

    pub fn from_json(json: Json) -> Result<ChatJson, ChatJsonError> {
        match json {
            Json::Object(map) => {
                let mut result = ChatJson::from("");
                for (key, value) in map {
                    println!("{:?}: {:?}", key, value);
                    match &key[..] {
                        "text" => {
                            type_check!(&key => value, String(string) {
                                result.msg = Message::PlainText(string);
                            });
                        }
                        "translate" => {
                            type_check!(&key => value, String(string) {
                                if let Message::Translatable(ref mut translatable, _) = result.msg {
                                    *translatable = string;
                                } else {
                                    result.msg = Message::Translatable(string, vec![]);
                                }
                            });
                        }
                        "with" => {
                            type_check!(&key => value, Array(with_json) {
                                let with = try!(with_json.into_iter().map(ChatJson::from_json).collect());
                                if let Message::Translatable(_, ref mut with_field) = result.msg {
                                    *with_field = with;
                                } else {
                                    result.msg = Message::Translatable("".to_string(), with);
                                }
                            });
                        }
                        "score" => {
                            type_check!(&key => value, Object(score) {
                                let name: String = match score.get("name") {
                                    Some(&Json::String(ref string)) => string.clone(),
                                    _ => return Err(ChatJsonError::InvalidScore)
                                };
                                let objective: String = match score.get("objective") {
                                    Some(&Json::String(ref string)) => string.clone(),
                                    _ => return Err(ChatJsonError::InvalidScore)
                                };
                                // error when score contains additional fields
                                if score.keys().any(|k| k != "name" && k != "objective") {
                                    return Err(ChatJsonError::InvalidScore)
                                }
                                result.msg = Message::Score { name: name, objective: objective };
                            });
                        }
                        "selector" => {
                            type_check!(&key => value, String(sel) {
                                result.msg = Message::Selector(try!(EntitySelector::from_str(&sel)));
                            });
                        }
                        "insertion" => {
                            type_check!(&key => value, String(string) {
                                result.insertion = Some(string);
                            });
                        }
                        "color" => {
                            type_check!(&key => value, String(string) {
                                result.color = match Color::from_str(&string) {
                                    Err(_) => return Err(ChatJsonError::InvalidColor(string)),
                                    Ok(c) => Some(c)
                                }
                            });
                        }
                        // Handle all of the different format strings.
                        "bold"|"italic"|"underlined"|"strikethrough"|"obfuscated"|"reset"|"random" => {
                            type_check!(&key => value, Boolean(b) {
                                if b == true {
                                    result.formats.insert(Format::from_string(&key).unwrap());
                                }
                            });
                        }
                        // Handle the JSON format of click events.
                        "clickEvent" => {
                            type_check!(&key => value, Object(event) {
                                // Get the `value` first.
                                let val: String = match event.get("value") {
                                    Some(&Json::String(ref string)) => string.clone(),
                                    _ => return Err(ChatJsonError::InvalidClickEvent)
                                };
                                // Handle the different click events.
                                if let Some(&Json::String(ref string)) = event.get("action") {
                                    result.click_event = match &string[..] {
                                        "open_url" => Some(ClickEvent::OpenUrl(val)),
                                        "open_file" => Some(ClickEvent::OpenFile(val)),
                                        "run_command" => Some(ClickEvent::RunCommand(val)),
                                        "suggest_command" => Some(ClickEvent::SuggestCommand(val)),
                                        _ => return Err(ChatJsonError::InvalidClickEvent)
                                    };
                                } else {
                                    return Err(ChatJsonError::InvalidClickEvent);
                                }
                                // error when clickEvent contains additional fields
                                if event.keys().any(|k| k != "action" && k != "value") {
                                    return Err(ChatJsonError::InvalidClickEvent)
                                }
                            });
                        }
                        // Handle the JSON format of hover events.
                        "hoverEvent" => {
                            type_check!(&key => value, Object(event) {
                                // Get the `value` first.
                                let val: String = match event.get("value") {
                                    Some(&Json::String(ref string)) => string.clone(),
                                    _ => return Err(ChatJsonError::InvalidHoverEvent)
                                };
                                // Handle the different click events.
                                if let Some(&Json::String(ref string)) = event.get("action") {
                                    result.hover_event = match &string[..] {
                                        "show_text" => Some(HoverEvent::Text(val)),
                                        "show_achievement" => Some(HoverEvent::Achievement(val)),
                                        "show_item" => Some(HoverEvent::Item(val)),
                                        _ => return Err(ChatJsonError::InvalidHoverEvent)
                                    };
                                } else {
                                    return Err(ChatJsonError::InvalidHoverEvent);
                                }
                                // error when clickEvent contains additional fields
                                if event.keys().any(|k| k != "action" && k != "value") {
                                    return Err(ChatJsonError::InvalidHoverEvent)
                                }
                            });
                        }
                        "extra" => {
                            type_check!(&key => value, Array(extra) {
                                result.extra = try!(extra.into_iter().map(|elt| ChatJson::from_json(elt)).collect());
                            });
                        }
                        v => return Err(ChatJsonError::UnknownField(v.to_string()))
                    };
                }
                Ok(result)
            }
            Json::Array(array) => {
                Ok(ChatJson { extra: try!(array.into_iter().map(|elt| ChatJson::from_json(elt)).collect()), ..ChatJson::from("") })
            }
            Json::String(string) => Ok(ChatJson::from(string)),
            v => Err(ChatJsonError::InvalidRootType(JsonType::from(v)))
        }
    }
}

impl From<String> for ChatJson {
    fn from(msg: String) -> ChatJson {
        ChatJson {
            msg: Message::PlainText(msg),
            extra: vec![],
            color: None,
            formats: BTreeSet::new(),
            click_event: None,
            hover_event: None,
            insertion: None
        }
    }
}

impl<'a> From<&'a str> for ChatJson {
    fn from(msg: &str) -> ChatJson {
        ChatJson::from(msg.to_string())
    }
}

impl ToJson for ChatJson {
    fn to_json(&self) -> Json {
        if let ChatJson { msg: Message::PlainText(ref text), ref extra, color: None, ref formats, click_event: None, hover_event: None, insertion: None } = *self {
            if extra.len() == 0 && *formats == BTreeSet::new() {
                // No formatting or other fancy stuff is used, just return the JSON string
                return text.to_json();
            }
        }

        let mut d = BTreeMap::new();

        match self.msg {
            Message::PlainText(ref text) => {
                d.insert("text".to_string(), text.to_json());
            }
            Message::Score { ref name, ref objective } => {
                let mut score = json::Object::default();
                score.insert("name".to_owned(), Json::String(name.clone()));
                score.insert("objective".to_owned(), Json::String(objective.clone()));
                d.insert("score".to_string(), Json::Object(score));
            }
            Message::Translatable(ref translate, ref with) => {
                d.insert("translate".to_string(), translate.to_json());
                d.insert("with".to_string(), with.to_json());
            }
            Message::Selector(ref sel) => {
                d.insert("selector".to_string(), Json::String(String::from(sel)));
            }
        };

        for format in &self.formats {
            d.insert(format.to_string(), Json::Boolean(true));
        }

        if self.extra.len() > 0 {
            d.insert("extra".to_string(), self.extra.to_json());
        }
        if let Some(ref color) = self.color {
            d.insert("color".to_string(), color.to_json());
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

impl Encodable for ChatJson {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        self.to_json().encode(s)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    PlainText(String),
    Score { name: String, objective: String },
    Translatable(String, Vec<ChatJson>),
    Selector(EntitySelector)
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

    pub fn from_string(string: &str) -> Option<Format> {
        match string {
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
    use types::consts::Color;
    use std::io;
    use rustc_serialize::json::{Builder, ToJson};

    #[test]
    fn chat_plain() {
        let msg = ChatJson::from("Hello, world!");
        let blob = r#"{
            "text": "Hello, world!"
        }"#;
        let parsed = ChatJson::from_reader(&mut io::Cursor::new(blob.as_bytes())).unwrap();
        assert_eq!(&msg, &parsed);
    }

    #[test]
    fn chat_invalid_field() {
        let blob = r#"{
            "text": true
        }"#;
        let parsed = ChatJson::from_reader(&mut io::Cursor::new(blob.as_bytes()));
        match parsed {
            Err(ChatJsonError::InvalidFieldType { name, expected: JsonType::String, found: JsonType::Boolean }) => {
                assert_eq!(&name, "text");
            }
            Err(_) => panic!("Wrong error type"),
            Ok(_) => panic!("Should return error on invalid field type")
        }
    }

    #[test]
    fn chat_with_events() {
        let mut msg = ChatJson::from("Hello, world!");
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
        let parsed = ChatJson::from_reader(&mut io::Cursor::new(blob.as_bytes())).unwrap();
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

        let parsed = ChatJson::from_reader(&mut io::Cursor::new(blob.as_bytes()));
        println!("{:?}", parsed);
    }
}
