extern crate rustc_serialize;

use rustc_serialize::json;
use std::collections::BTreeMap;

pub type Handler<'a,T> = Box<FnMut(String, json::JsonEvent, &mut json::Parser<T>)+'a>;

pub fn dummy_handler<'a,T: Iterator<Item=char>>() -> Handler<'a,T> {
    Box::new(|_,first,parser| {
        // println!("Dummy read...");
        read_value(first,parser);
    })
}

pub struct StreamReader<'a,T> {
    handlers: BTreeMap<String,Handler<'a,T>>,
    default_handler: Handler<'a,T>,
}

impl <'a,T: Iterator<Item=char>> StreamReader<'a,T> {
    pub fn new() -> Self {
        StreamReader {
            handlers: BTreeMap::new(),
            default_handler: dummy_handler(),
        }
    }

    pub fn set_default_handler(&mut self, handler: Handler<'a,T>) {
        self.default_handler = handler;
    }

    pub fn set_handler(&mut self, name: String, handler: Handler<'a,T>) {
        self.handlers.insert(name, handler);
    }


    pub fn read_object(&mut self, parser: &mut json::Parser<T>) {
        // println!("Reading new object!");

        loop {
            match parser.next() {
                None | Some(json::JsonEvent::ObjectEnd) => return,
                Some(token) => {
                    // println!("Token: {:?}", token);
                    let key = match parser.stack().top() {
                        Some(json::StackElement::Key(k)) => k.to_string(),
                        Some(thing) => panic!("invalid state: {:?}", thing),
                        None => panic!("no stack???"),
                    };
                    // println!("Key was: {}", &key);
                    let handler = self.handlers.get_mut(&key).unwrap_or(&mut self.default_handler);
                    handler(key, token, parser);
                }
            }
        }
    }
}

pub fn next<T: Iterator<Item=char>>(parser: &mut json::Parser<T>) -> json::JsonEvent {
    parser.next().expect("unpexpected end of stream")
}

pub fn copy_handler<'a,T:Iterator<Item=char>>(target: &'a mut json::Object) -> Handler<'a,T> {
    Box::new(move |key,first,parser| {
        target.insert(key, read_value(first,parser));
    })
}

/// Builds a special king of handler that only reads arrays
pub fn array_handler<'a,F: 'a+FnMut(json::Json),T:Iterator<Item=char>>(mut object_handler: F) -> Handler<'a,T> {
    Box::new(move |_,first,parser| {
        if first != json::JsonEvent::ArrayStart {
            panic!("non-array found");
        }

        loop {
            // Read values
            match parser.next() {
                None | Some(json::JsonEvent::ArrayEnd) => return,
                Some(token) => object_handler(read_value(token, parser)),
            }
        }
    })
}

pub fn read_value<T: Iterator<Item=char>>(first: json::JsonEvent, parser: &mut json::Parser<T>) -> json::Json {
    // println!("Reading from {:?}", first);
    match first {
        json::JsonEvent::ObjectStart => json::Json::Object(read_object(parser)),
        json::JsonEvent::ArrayStart => json::Json::Array(read_array(parser)),
        json::JsonEvent::BooleanValue(b) => json::Json::Boolean(b),
        json::JsonEvent::I64Value(i) => json::Json::I64(i),
        json::JsonEvent::U64Value(u) => json::Json::U64(u),
        json::JsonEvent::F64Value(f) => json::Json::F64(f),
        json::JsonEvent::StringValue(s) => json::Json::String(s),
        json::JsonEvent::NullValue => json::Json::Null,
        token => { println!("unexpected token: {:?}", token); json::Json::Null },
    }
}

pub fn read_array<T: Iterator<Item=char>>(parser: &mut json::Parser<T>) -> json::Array {
    let mut result = json::Array::new();
    // We don't really care about the key here, so String::new() is enough
    array_handler(|item| result.push(item))(String::new(), json::JsonEvent::ArrayStart, parser);
    result
}

pub fn read_object<T: Iterator<Item=char>>(parser: &mut json::Parser<T>) -> json::Object {
    let mut result = json::Object::new();

    {
        let mut reader = StreamReader::new();
        reader.set_default_handler(Box::new(|name,first,parser| {
            result.insert(name, read_value(first,parser));
        }));
        reader.read_object(parser);
    }

    result
}
