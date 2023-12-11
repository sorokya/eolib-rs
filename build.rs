use glob::glob;
use std::fs::File;
use std::io::BufReader;
use xml::reader::{EventReader, XmlEvent};

macro_rules! p {
    ($($tokens: tt)*) => {
        println!("cargo:warning={}", format!($($tokens)*))
    }
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

enum ProtocolType {
    Struct,
    Enum,
    Packet,
}

fn parse_protocol_file(path: &std::path::Path) -> std::io::Result<()> {
    let file = File::open(path)?;
    let file = BufReader::new(file);

    let parser = EventReader::new(file);

    let mut protocol_type: Option<ProtocolType> = None;
    let mut protocol_enum: Option<Enum> = None;

    for e in parser {
        match e {
            Ok(XmlEvent::StartElement {
                name, attributes, ..
            }) => {
                p!("{}: {:?}", name, attributes);

                match name.local_name.as_str() {
                    "enum" => {
                        let name = match attributes
                            .iter()
                            .find(|a| a.name.local_name == "name")
                            .map(|a| a.name.local_name.to_owned())
                        {
                            Some(name) => name,
                            None => {
                                p!("Enum without name");
                                continue;
                            }
                        };

                        let r#type = match attributes
                            .iter()
                            .find(|a| a.name.local_name == "type")
                            .map(|a| a.name.local_name.to_owned())
                        {
                            Some(r#type) => match r#type.as_str() {
                                "byte" => EnumType::Byte,
                                "char" => EnumType::Char,
                                "short" => EnumType::Short,
                                "three" => EnumType::Three,
                                "int" => EnumType::Int,
                                _ => {
                                    p!("Unknown enum type: {}", r#type);
                                    continue;
                                }
                            },
                            None => {
                                p!("Enum without type");
                                continue;
                            }
                        };
                        protocol_type = Some(ProtocolType::Enum);
                        protocol_enum = Some(Enum::new(name, r#type));
                    }
                    _ => {}
                }
            }
            Ok(XmlEvent::Characters(value)) => {
                p!("Value: {}", value);
            }
            Err(e) => {
                p!("Error: {}", e);
                break;
            }
            _ => {}
        }
    }
    Ok(())
}

#[derive(Debug, Default)]
struct Enum {
    name: String,
    r#type: EnumType,
    fields: Vec<EnumField>,
}

impl Enum {
    pub fn new(name: String, r#type: EnumType) -> Self {
        Self {
            name,
            r#type,
            fields: Vec::new(),
        }
    }

    pub fn add_field(&mut self, name: String, value: u32) {
        self.fields.push(EnumField { name, value });
    }
}

#[derive(Debug)]
enum EnumType {
    Byte,
    Char,
    Short,
    Three,
    Int,
}

impl Default for EnumType {
    fn default() -> Self {
        Self::Byte
    }
}

#[derive(Debug, Default)]
struct EnumField {
    pub name: String,
    pub value: u32,
}
