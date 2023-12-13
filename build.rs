use glob::glob;
use serde::Deserialize;
use std::{fs::File, io::Read};

macro_rules! p {
    ($($tokens: tt)*) => {
        println!("cargo:warning={}", format!($($tokens)*))
    }
}

#[derive(Debug, Deserialize)]
struct Protocol {
    #[serde(rename = "$value", default)]
    pub elements: Vec<Element>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Element {
    Enum(Enum),
    Struct(Struct),
    Packet(Packet),
}

#[derive(Debug, Deserialize)]
struct Enum {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@type")]
    pub data_type: String,
    #[serde(rename = "$value", default)]
    pub elements: Vec<EnumElement>,
}

#[derive(Debug, Deserialize)]
enum EnumElement {
    #[serde(rename = "comment")]
    Comment(String),
    #[serde(rename = "value")]
    Value(EnumValue),
}

#[derive(Debug, Deserialize)]
struct EnumValue {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "comment")]
    pub comment: Option<String>,
    #[serde(rename = "$text")]
    pub value: i32,
}

#[derive(Debug, Deserialize)]
struct Struct {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "$value", default)]
    pub elements: Vec<StructElement>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum StructElement {
    Break,
    Chunked(Chunked),
    Comment(String),
    Dummy(Dummy),
    Field(Field),
    Array(Array),
    Length(Length),
    Switch(Switch),
}

#[derive(Debug, Deserialize)]
struct Chunked {
    #[serde(rename = "$value", default)]
    pub elements: Vec<StructElement>,
}

#[derive(Debug, Deserialize)]
struct Field {
    #[serde(rename = "@name")]
    pub name: Option<String>,
    #[serde(rename = "@type")]
    pub data_type: String,
    #[serde(rename = "$value", default)]
    pub value: Option<String>,
    pub comment: Option<String>,
    #[serde(rename = "padded")]
    pub padded: Option<bool>,
    #[serde(rename = "optional")]
    pub length: Option<String>,
}

fn default_as_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
struct Array {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@type")]
    pub data_type: String,
    #[serde(rename = "@length")]
    pub length: Option<String>,
    #[serde(rename = "optional")]
    pub optional: Option<bool>,
    #[serde(rename = "delimited")]
    pub delimited: Option<bool>,
    #[serde(rename = "@trailing-delimiter")]
    #[serde(default = "default_as_true")]
    pub trailing_delimiter: bool,
    pub comment: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Length {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@type")]
    pub data_type: String,
    #[serde(rename = "@length")]
    pub optional: Option<bool>,
    #[serde(rename = "@offset")]
    pub offset: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct Dummy {
    #[serde(rename = "@type")]
    pub data_type: String,
    #[serde(rename = "$value", default)]
    pub value: String,
}

#[derive(Debug, Deserialize)]
struct Switch {
    #[serde(rename = "@field")]
    pub field: String,
    #[serde(rename = "$value", default)]
    pub cases: Vec<Case>,
}

#[derive(Debug, Deserialize)]
#[serde(rename = "case")]
struct Case {
    #[serde(rename = "@default")]
    pub default: Option<bool>,
    #[serde(rename = "@value")]
    pub value: Option<String>,
    #[serde(rename = "$value", default)]
    pub elements: Option<Vec<StructElement>>,
}

#[derive(Debug, Deserialize)]
struct Packet {
    #[serde(rename = "@action")]
    pub action: String,
    #[serde(rename = "@family")]
    pub family: String,
    #[serde(rename = "$value", default)]
    pub elements: Vec<StructElement>,
}

fn main() {
    println!("cargo:rerun-if-changed=eo-protocol/xml");

    // find all protocol.xml files in the xml directory recursively
    for entry in glob("eo-protocol/xml/**/protocol.xml").expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                parse_protocol_file(&path).expect("Failed to parse protocol file");
            }
            Err(e) => println!("{:?}", e),
        }
    }
}

fn parse_protocol_file(path: &std::path::Path) -> std::io::Result<()> {
    let mut file = File::open(path)?;
    let mut xml = String::new();
    file.read_to_string(&mut xml)?;

    let _protocol: Protocol = match quick_xml::de::from_str(&xml) {
        Ok(protocol) => protocol,
        Err(err) => {
            p!("error: {} in {}", err, path.to_string_lossy());
            return Ok(());
        }
    };

    Ok(())
}
