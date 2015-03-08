use std::collections::BTreeMap;

use rustc_serialize::json::{Json, ToJson};

pub struct ChatJson {
    pub msg: Message,
    pub extra: Option<Vec<Json>>,
    pub color: Option<Color>,
    pub formats: Option<Vec<Format>>,
    pub click_event: Option<ClickEvent>,
    pub hover_event: Option<HoverEvent>,
    pub insertion: Option<String>
}

impl ChatJson {
    pub fn msg(msg: String) -> ChatJson {
        ChatJson {
            msg: Message::PlainText(msg), extra: None, color: None, formats: None,
            click_event: None, hover_event: None, insertion: None
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

pub enum Message {
    PlainText(String),
    Score(String, String),
    Translatable,
    Selector
}

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

#[derive(Copy, FromPrimitive)]
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
    pub fn to_code(&self) -> u8 {
        (*self) as u8
    }

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

pub enum Format {
    Bold, Underlined, Strikethrough, Italic, Obfuscated, Random, Reset
}

#[cfg(test)]
mod test {
    use super::*;
    use rustc_serialize::json::ToJson;

    #[test]
    fn chat_plain() {
        let msg = ChatJson::msg("Hello, world!".to_string());
        println!("{}", msg.to_json());
    }
}
