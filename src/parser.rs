use std::io::BufReader;
use std::fs::File;
use std::path::PathBuf;
use xml::attribute::OwnedAttribute;
use xml::common::{Error, Position};
use xml::name::OwnedName;
use xml::reader::EventReader;
use xml::reader::events::XmlEvent::{self, StartElement, EndElement, EndDocument};

use library::*;

type Reader = EventReader<BufReader<File>>;
type Attributes = Vec<OwnedAttribute>;

macro_rules! error {
    ($msg:expr, $obj:expr) => (
        Error::new($obj, format!("{}:{} {}", file!(), line!(), $msg))
    )
}

macro_rules! xml_try {
    ($event:expr, $pos:expr) => (
        match $event {
            XmlEvent::Error(e) => return Err(e),
            EndDocument => return Err(error!("Unexpected end of document", $pos)),
            _ => (),
        }
    )
}

impl Library {
    pub fn read_file(&mut self, dir: &str, lib: &str) {
        let name = make_file_name(dir, lib);
        let display_name = name.to_string_lossy().into_owned();
        let file = BufReader::new(File::open(&name)
            .unwrap_or_else(|e| panic!("{} - {}", e, name.to_string_lossy())));
        let mut parser = EventReader::new(file);
        loop {
            let event = parser.next();
            match event {
                StartElement { name: OwnedName { ref local_name,
                                                 namespace: Some(ref namespace), .. }, .. }
                            if local_name == &"repository"
                            && namespace == &"http://www.gtk.org/introspection/core/1.0" => {
                    match self.read_repository(dir, &mut parser) {
                        Err(e) => panic!("{} in {}:{}",
                                         e.msg(), display_name, e.position()),
                        Ok(_) => (),
                    }
                }
                XmlEvent::Error(e) => panic!("{} in {}:{}",
                                             e.msg(), display_name, e.position()),
                EndDocument => break,
                _ => continue,
            }
        }
    }

    fn read_repository(&mut self, dir: &str, parser: &mut Reader) -> Result<(), Error> {
        loop {
            let event = parser.next();
            match event {
                StartElement { name, attributes, .. } => {
                    match name.local_name.as_ref() {
                        "include" => {
                            if let (Some(lib), Some(ver)) =
                                (attributes.get("name"), attributes.get("version")) {
                                if !self.has_namespace(lib) {
                                    let lib = format!("{}-{}", lib, ver);
                                    self.read_file(dir, &lib);
                                }
                            }
                            try!(ignore_element(parser));
                        }
                        "namespace" => try!(self.read_namespace(parser, &attributes)),
                        _ => try!(ignore_element(parser)),
                    }
                }
                EndElement { .. } => return Ok(()),
                _ => xml_try!(event, parser),
            }
        }
    }

    fn read_namespace(&mut self, parser: &mut Reader,
                      attrs: &Attributes) -> Result<(), Error> {
        let name = try!(attrs.get("name").ok_or_else(|| error!("Missing namespace name", parser)));
        let ns_id = self.get_namespace(name);
        self.namespace_mut(ns_id).name = name.into();
        //println!("Reading {}-{}", namespace, attrs.get("version").unwrap());
        loop {
            let event = parser.next();
            match event {
                StartElement { name, attributes, .. } => {
                    //println!("<{} name={:?}>", name.local_name, attributes.get("name"));
                    match name.local_name.as_ref() {
                        "class" => {
                            //println!("class {}", attributes.get("name").unwrap());
                            try!(self.read_class(parser, ns_id, &attributes));
                        }
                        "record" => {
                            try!(self.read_record(parser, ns_id, &attributes));
                        }
                        "union" => {
                            try!(self.read_union(parser, ns_id, &attributes));
                        }
                        "interface" => {
                            try!(self.read_interface(parser, ns_id, &attributes));
                        }
                        "callback" => {
                            try!(self.read_callback(parser, ns_id, &attributes));
                        }
                        "bitfield" => {
                            try!(self.read_bitfield(parser, ns_id, &attributes));
                        }
                        "enumeration" => {
                            try!(self.read_enumeration(parser, ns_id, &attributes));
                        }
                        "function" => {
                            try!(self.read_global_function(parser, ns_id, &attributes));
                        }
                        "constant" => {
                            try!(self.read_constant(parser, ns_id, &attributes));
                        }
                        "alias" => {
                            try!(self.read_alias(parser, ns_id, &attributes));
                        }
                        _ => {
                            println!("<{} name={:?}>", name.local_name, attributes.get("name"));
                            try!(ignore_element(parser));
                        }
                    }
                }
                EndElement { .. } => break,
                _ => xml_try!(event, parser),
            }
        }
        Ok(())
    }

    fn read_class(&mut self, parser: &mut Reader,
                  ns_id: u16, attrs: &Attributes) -> Result<(), Error> {
        let name = try!(attrs.get("name").ok_or_else(|| error!("Missing class name", parser)));
        let type_name = attrs.get("type-name").unwrap_or(name);
        let mut fns = Vec::new();
        loop {
            let event = parser.next();
            match event {
                StartElement { name, attributes, .. } => {
                    match name.local_name.as_ref() {
                        "constructor" | "function" | "method" => {
                            fns.push(try!(
                                self.read_function(parser, ns_id, &attributes)));
                        }
                        "field" | "property" | "implements"
                            | "signal" | "virtual-method" => try!(ignore_element(parser)),
                        "doc" | "doc-deprecated" => try!(ignore_element(parser)),
                        x => return Err(error!(format!("Unexpected element <{}>", x), parser)),
                    }
                }
                EndElement { .. } => break,
                _ => xml_try!(event, parser),
            }
        }

        let typ = Type::Class(
            Class {
                name: type_name.into(),
                functions: fns,
            });
        self.add_type(ns_id, name, typ);
        Ok(())
    }

    fn read_record(&mut self, parser: &mut Reader,
                  ns_id: u16, attrs: &Attributes) -> Result<(), Error> {
        let name = try!(attrs.get("name").ok_or_else(|| error!("Missing record name", parser)));
        let type_name = attrs.get("type-name").unwrap_or(name);
        let mut fns = Vec::new();
        loop {
            let event = parser.next();
            match event {
                StartElement { name, attributes, .. } => {
                    match name.local_name.as_ref() {
                        "constructor" | "function" | "method" => {
                            fns.push(try!(
                                self.read_function(parser, ns_id, &attributes)));
                        }
                        "field" | "union" => try!(ignore_element(parser)),
                        "doc" | "doc-deprecated" => try!(ignore_element(parser)),
                        x => return Err(error!(format!("Unexpected element <{}>", x), parser)),
                    }
                }
                EndElement { .. } => break,
                _ => xml_try!(event, parser),
            }
        }

        if attrs.get("is-gtype-struct").is_some() {
            return Ok(());
        }

        let typ = Type::Record(
            Record {
                name: type_name.into(),
                functions: fns,
            });
        self.add_type(ns_id, name, typ);
        Ok(())
    }

    fn read_union(&mut self, parser: &mut Reader,
                  ns_id: u16, attrs: &Attributes) -> Result<(), Error> {
        let name = try!(attrs.get("name").ok_or_else(|| error!("Missing union name", parser)));
        let type_name = attrs.get("type-name").unwrap_or(name);
        let mut fields = Vec::new();
        let mut fns = Vec::new();
        loop {
            let event = parser.next();
            match event {
                StartElement { name, attributes, .. } => {
                    match name.local_name.as_ref() {
                        "field" => {
                            fields.push(try!(
                                self.read_field(parser, ns_id, &attributes)));
                        }
                        "constructor" | "function" | "method" => {
                            fns.push(try!(
                                self.read_function(parser, ns_id, &attributes)));
                        }
                        "record" => try!(ignore_element(parser)),
                        "doc" | "doc-deprecated" => try!(ignore_element(parser)),
                        x => return Err(error!(format!("Unexpected element <{}>", x), parser)),
                    }
                }
                EndElement { .. } => break,
                _ => xml_try!(event, parser),
            }
        }

        let typ = Type::Union(
            Union {
                name: type_name.into(),
                fields: fields,
                functions: fns,
            });
        self.add_type(ns_id, name, typ);
        Ok(())
    }

    fn read_field(&mut self, parser: &mut Reader, ns_id: u16,
                  attrs: &Attributes) -> Result<Field, Error> {
        let name = try!(attrs.get("name").ok_or_else(|| error!("Missing field name", parser)));
        let mut typ = None;
        loop {
            let event = parser.next();
            match event {
                StartElement { name, attributes, .. } => {
                    match name.local_name.as_ref() {
                        "type" | "array" => {
                            if typ.is_some() {
                                return Err(error!("Too many <type> elements", parser));
                            }
                            typ = Some(try!(
                                self.read_type(parser, ns_id, &name, &attributes)));
                        }
                        "doc" | "doc-deprecated" => try!(ignore_element(parser)),
                        x => return Err(error!(format!("Unexpected element <{}>", x), parser)),
                    }
                }
                EndElement { .. } => break,
                _ => xml_try!(event, parser),
            }
        }
        if let Some(typ) = typ {
            Ok(Field {
                name: name.into(),
                typ: typ,
            })
        }
        else {
            Err(error!("Missing <type> element", parser))
        }
    }

    fn read_callback(&mut self, parser: &mut Reader, ns_id: u16,
                     attrs: &Attributes) -> Result<(), Error> {
        let name = try!(attrs.get("name").ok_or_else(|| error!("Missing callback name", parser)));
        let func = try!(self.read_function(parser, ns_id, attrs));
        let cb = Type::Callback(func);
        self.add_type(ns_id, name, cb);
        Ok(())
    }

    fn read_interface(&mut self, parser: &mut Reader,
                      ns_id: u16, attrs: &Attributes) -> Result<(), Error> {
        let name = try!(attrs.get("name").ok_or_else(|| error!("Missing interface name", parser)));
        let type_name = attrs.get("type-name").unwrap_or(name);
        let mut fns = Vec::new();
        loop {
            let event = parser.next();
            match event {
                StartElement { name, attributes, .. } => {
                    match name.local_name.as_ref() {
                        "constructor" | "function" | "method" =>
                            fns.push(try!( self.read_function(parser, ns_id, &attributes))),
                        "doc" | "doc-deprecated" => try!(ignore_element(parser)),
                        _ => try!(ignore_element(parser)),
                    }
                }
                EndElement { .. } => break,
                _ => xml_try!(event, parser),
            }
        }

        let typ = Type::Interface(
            Interface {
                name: type_name.into(),
                functions: fns,
            });
        self.add_type(ns_id, name, typ);
        Ok(())
    }

    fn read_bitfield(&mut self, parser: &mut Reader, ns_id: u16,
                     attrs: &Attributes) -> Result<(), Error> {
        let name = try!(attrs.get("name").ok_or_else(|| error!("Missing bitfield name", parser)));
        let type_name = attrs.get("type-name").unwrap_or(name);
        let mut members = Vec::new();
        let mut fns = Vec::new();
        loop {
            let event = parser.next();
            match event {
                StartElement { name, attributes, .. } => {
                    match name.local_name.as_ref() {
                        "member" => {
                            members.push(try!(
                                self.read_member(parser, &attributes)));
                        }
                        "constructor" | "function" | "method" => {
                            fns.push(try!(
                                self.read_function(parser, ns_id, &attributes)));
                        }
                        "doc" | "doc-deprecated" => try!(ignore_element(parser)),
                        x => return Err(error!(format!("Unexpected element <{}>", x), parser)),
                    }
                }
                EndElement { .. } => break,
                _ => xml_try!(event, parser),
            }
        }

        let typ = Type::Bitfield(
            Bitfield {
                name: type_name.into(),
                members: members,
                functions: fns,
            });
        self.add_type(ns_id, name, typ);
        Ok(())
    }

    fn read_enumeration(&mut self, parser: &mut Reader, ns_id: u16,
                        attrs: &Attributes) -> Result<(), Error> {
        let name = try!(attrs.get("name").ok_or_else(|| error!("Missing enumeration name", parser)));
        let type_name = attrs.get("type-name").unwrap_or(name);
        let mut members = Vec::new();
        let mut fns = Vec::new();
        loop {
            let event = parser.next();
            match event {
                StartElement { name, attributes, .. } => {
                    match name.local_name.as_ref() {
                        "member" => {
                            members.push(try!(
                                self.read_member(parser, &attributes)));
                        }
                        "constructor" | "function" | "method" => {
                            fns.push(try!(
                                self.read_function(parser, ns_id, &attributes)));
                        }
                        "doc" | "doc-deprecated" => try!(ignore_element(parser)),
                        x => return Err(error!(format!("Unexpected element <{}>", x), parser)),
                    }
                }
                EndElement { .. } => break,
                _ => xml_try!(event, parser),
            }
        }

        let typ = Type::Enumeration(
            Enumeration {
                name: type_name.into(),
                members: members,
                functions: fns,
            });
        self.add_type(ns_id, name, typ);
        Ok(())
    }

    fn read_global_function(&mut self, parser: &mut Reader, ns_id: u16,
                            attrs: &Attributes) -> Result<(), Error> {
        let func = try!(self.read_function(parser, ns_id, attrs));
        self.namespace_mut(ns_id).functions.push(func);
        Ok(())
    }

    fn read_constant(&mut self, parser: &mut Reader, ns_id: u16,
                     attrs: &Attributes) -> Result<(), Error> {
        let name = try!(attrs.get("name").ok_or_else(|| error!("Missing constant name", parser)));
        let value = try!(attrs.get("value").ok_or_else(|| error!("Missing constant value", parser)));
        let mut typ = None;
        loop {
            let event = parser.next();
            match event {
                StartElement { name, attributes, .. } => {
                    match name.local_name.as_ref() {
                        "type" | "array" => {
                            if typ.is_some() {
                                return Err(error!("Too many <type> elements", parser));
                            }
                            typ = Some(try!(
                                self.read_type(parser, ns_id, &name, &attributes)));
                        }
                        "doc" | "doc-deprecated" => try!(ignore_element(parser)),
                        x => return Err(error!(format!("Unexpected element <{}>", x), parser)),
                    }
                }
                EndElement { .. } => break,
                _ => xml_try!(event, parser),
            }
        }
        if let Some(typ) = typ {
            self.namespace_mut(ns_id).constants.push(
                Constant {
                    name: name.into(),
                    typ: typ,
                    value: value.into(),
                });
            Ok(())
        }
        else {
            Err(error!("Missing <type> element", parser))
        }
    }

    fn read_alias(&mut self, parser: &mut Reader, ns_id: u16,
                     attrs: &Attributes) -> Result<(), Error> {
        let name = try!(attrs.get("name").ok_or_else(|| error!("Missing constant name", parser)));
        let c_identifier = attrs.get("type").unwrap_or(name);
        let mut inner = None;
        loop {
            let event = parser.next();
            match event {
                StartElement { name, attributes, .. } => {
                    match name.local_name.as_ref() {
                        "type" | "array" => {
                            if inner.is_some() {
                                return Err(error!("Too many <type> elements", parser));
                            }
                            inner = Some(try!(
                                self.read_type(parser, ns_id, &name, &attributes)));
                        }
                        "doc" | "doc-deprecated" => try!(ignore_element(parser)),
                        x => return Err(error!(format!("Unexpected element <{}>", x), parser)),
                    }
                }
                EndElement { .. } => break,
                _ => xml_try!(event, parser),
            }
        }
        if let Some(inner) = inner {
            let typ = Type::Alias(
                Alias {
                    name: name.into(),
                    c_identifier: c_identifier.into(),
                    typ: inner,
                });
            self.add_type(ns_id, name, typ);
            Ok(())
        }
        else {
            Err(error!("Missing <type> element", parser))
        }
    }

    fn read_member(&self, parser: &mut Reader, attrs: &Attributes) -> Result<Member, Error> {
        let name = try!(attrs.get("name").ok_or_else(|| error!("Missing member name", parser)));
        let value = try!(attrs.get("value").ok_or_else(|| error!("Missing member value", parser)));
        let c_identifier = attrs.get("identifier").map(|x| x.into());
        loop {
            let event = parser.next();
            match event {
                StartElement { name, attributes, .. } => {
                    match (name.local_name.as_ref(), attributes.get("name")) {
                        /*
                        ("attribute", Some("c:identifier")) => {
                            let value = try!(attributes.get("value")
                                .ok_or_else(|| error!("Missing attribute value", parser)));
                            c_identifier = Some(value.into());
                        }
                        */
                        ("doc", _) | ("doc-deprecated", _) => try!(ignore_element(parser)),
                        (x, _) => return Err(error!(format!("Unexpected element <{}>", x), parser)),
                    }
                }
                EndElement { .. } => break,
                _ => xml_try!(event, parser),
            }
        }
        Ok(Member {
            name: name.into(),
            value: value.into(),
            c_identifier: c_identifier.unwrap_or_else(|| name.into()),
        })
    }

    fn read_function(&mut self, parser: &mut Reader, ns_id: u16,
                     attrs: &Attributes) -> Result<Function, Error> {
        let name = try!(attrs.get("name").ok_or_else(|| error!("Missing function name", parser)));
        let c_identifier = attrs.get("identifier").unwrap_or(name);
        let mut params = Vec::new();
        let mut ret = None;
        loop {
            let event = parser.next();
            match event {
                StartElement { name, attributes, .. } => {
                    match name.local_name.as_ref() {
                        "parameters" => {
                            //params.append(&mut try!(self.read_parameters(parser, ns_id)));
                            try!(self.read_parameters(parser, ns_id)).into_iter()
                                .map(|p| params.push(p)).count();
                        }
                        "return-value" => {
                            if ret.is_some() {
                                return Err(error!("Too many <return-value> elements", parser));
                            }
                            ret = Some(try!(
                                self.read_parameter(parser, ns_id, &attributes)));
                        }
                        "doc" | "doc-deprecated" => try!(ignore_element(parser)),
                        x => return Err(error!(format!("Unexpected element <{}>", x), parser)),
                    }
                }
                EndElement { .. } => break,
                _ => xml_try!(event, parser),
            }
        }
        if let Some(ret) = ret {
            Ok(Function {
                name: name.into(),
                c_identifier: c_identifier.into(),
                parameters: params,
                ret: ret,
            })
        }
        else {
            Err(error!("Missing <return-value> element", parser))
        }
    }

    fn read_parameters(&mut self, parser: &mut Reader, ns_id: u16)
                    -> Result<Vec<Parameter>, Error> {
        let mut params = Vec::new();
        loop {
            let event = parser.next();
            match event {
                StartElement { name, attributes, .. } => {
                    match name.local_name.as_ref() {
                        "parameter" | "instance-parameter"  => {
                            let param = try!(
                                self.read_parameter(parser, ns_id, &attributes));
                            params.push(param);
                        }
                        x => return Err(error!(format!("Unexpected element <{}>", x), parser)),
                    }
                }
                EndElement { .. } => break,
                _ => xml_try!(event, parser),
            }
        }
        Ok(params)
    }

    fn read_parameter(&mut self, parser: &mut Reader, ns_id: u16,
                      attrs: &Attributes) -> Result<Parameter, Error> {
        let name = attrs.get("name").unwrap_or("");
        let transfer = try!(
            Transfer::by_name(attrs.get("transfer-ownership").unwrap_or("none"))
                .ok_or_else(|| error!("Unknown ownership transfer mode", parser)));
        let mut typ = None;
        let mut varargs = false;
        loop {
            let event = parser.next();
            match event {
                StartElement { name, attributes, .. } => {
                    match name.local_name.as_ref() {
                        "type" | "array" => {
                            if typ.is_some() {
                                return Err(error!("Too many <type> elements", parser));
                            }
                            typ = Some(try!(
                                self.read_type(parser, ns_id, &name, &attributes)));
                        }
                        "varargs" => {
                            varargs = true;
                            try!(ignore_element(parser));
                        }
                        "doc" => try!(ignore_element(parser)),
                        x => return Err(error!(format!("Unexpected element <{}>", x), parser)),
                    }
                }
                EndElement { .. } => break,
                _ => xml_try!(event, parser),
            }
        }
        if let Some(typ) = typ {
            Ok(Parameter {
                name: name.into(),
                typ: typ,
                transfer: transfer,
            })
        }
        else if varargs {
            Ok(Parameter {
                name: "".into(),
                typ: self.get_type(INTERNAL_NAMESPACE, "varargs"),
                transfer: Transfer::None,
            })
        }
        else {
            Err(error!("Missing <type> element", parser))
        }
    }

    fn read_type(&mut self, parser: &mut Reader, ns_id: u16,
                 name: &OwnedName, attrs: &Attributes) -> Result<TypeId, Error> {
        let start_pos = parser.position();
        let name =
            if name.local_name == "array" {
                "array"
            }
            else {
                try!(attrs.get("name").ok_or_else(|| error!("Missing type name", &start_pos)))
            };
        let mut inner = Vec::new();
        loop {
            let event = parser.next();
            match event {
                StartElement { name, attributes, .. } => {
                    match name.local_name.as_ref() {
                        "type" | "array" => {
                            inner.push(try!(
                                self.read_type(parser, ns_id, &name, &attributes)));
                        }
                        x => return Err(error!(format!("Unexpected element <{}>", x), parser)),
                    }
                }
                EndElement { .. } => break,
                _ => xml_try!(event, parser),
            }
        }
        if !inner.is_empty() {
            Ok(try!(Type::container(self, name, inner).ok_or_else(|| error!("Unknown container type", &start_pos))))
        }
        else {
            Ok(self.get_type(ns_id, name))
        }
    }
}

trait Get {
    fn get<'a>(&'a self, name: &str) -> Option<&'a str>;
}

impl Get for Attributes {
    fn get<'a>(&'a self, name: &str) -> Option<&'a str> {
        for attr in self {
            if attr.name.local_name == name {
                return Some(&attr.value);
            }
        }
        None
    }
}

fn ignore_element(parser: &mut Reader) -> Result<(), Error> {
    loop {
        let event = parser.next();
        match event {
            StartElement { .. } => try!(ignore_element(parser)),
            EndElement { .. } => return Ok(()),
            _ => xml_try!(event, parser),
        }
    }
}

fn make_file_name(dir: &str, name: &str) -> PathBuf {
    let mut path = PathBuf::from(dir);
    let name = format!("{}.gir", name);
    path.push(name);
    path
}
