//!
//! This crate exposes Rustic bindings to a gem of a C library called jsmn.
//! The raw bindings are available in the raw module, which are wrapped in
//! this module.
//!
//!
//! The jsmn library provides a very simple, fast JSON parser. I see it as a
//! great example of library design- it has two enums, two structs, and two functions,
//! and that is all. Its trivial to use, very fast, and has very few extra features.
//!
//!
//! This library only exposes a single function, jsmn_parse, because the jsmn_init
//! function is called when you crate a new JsmnParser.
//!
//!
//! To use this library, simply create a parser using JsmnParser::new()
//! and pass the parser, a JSON string, and a slice of JsmnToks to jsmn_parse.
//! The result will be that the slice will be filled out with tokens defining the
//! starting and ending offset of each JSON token in the given string.
//!
//!
//! Thats all there is to it! This crate is just intended to make jsmn easy to use
//! in Rust. There are other JSON parsers in Rust, and certainly Serde and HyperJson
//! are great crates, but I though jsmn deserved a place in the Rust ecosystem, so here
//! it is!
//!

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::mem::transmute;

pub mod raw;

/// The JSON object type. These enum values are identical to the jsmn library
/// enum jsmntype_t, but renamed to match Rust's conventions.
#[repr(u32)]
#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub enum JsmnType {
    #[default]
    JsmnUndefined = raw::jsmntype_t_JSMN_UNDEFINED,
    JsmnObject = raw::jsmntype_t_JSMN_OBJECT,
    JsmnArray = raw::jsmntype_t_JSMN_ARRAY,
    JsmnString = raw::jsmntype_t_JSMN_STRING,
    JsmnPrimitive = raw::jsmntype_t_JSMN_PRIMITIVE,
}

/// Error type from jsmn_parse. These enum values are identical to the jsmn library
/// enum jsmnerr_t, but renamed to match Rust's conventions.
#[repr(i32)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum JsmnErr {
    JsmErrorNoMem = raw::jsmnerr_JSMN_ERROR_NOMEM,
    JsmErrorInval = raw::jsmnerr_JSMN_ERROR_INVAL,
    JsmErrorPart = raw::jsmnerr_JSMN_ERROR_PART,
}

/// A JSON token structure, defining which type of JSON object it is, the starting
/// character, ending character, and size in bytes. All offsets are from the start
/// of the parsed string.
///
/// Note that if the parent-links feature is used, then this struct will
/// have the "parent" field, and otherwise it will not.
#[repr(C)]
#[derive(Debug, Copy, PartialEq)]
pub struct JsmnTok {
    pub typ: JsmnType,
    pub start: i32,
    pub end: i32,
    pub size: i32,
    #[cfg(feature = "parent-links")]
    pub parent: i32,
}

impl JsmnTok {
    pub fn new() -> Self {
        JsmnTok {
            typ: JsmnType::JsmnUndefined,
            start: 0,
            end: 0,
            size: 0,
            #[cfg(feature = "parent-links")]
            parent: 0,
        }
    }
}

impl Clone for JsmnTok {
    fn clone(&self) -> Self {
        *self
    }
}

impl Default for JsmnTok {
    fn default() -> Self {
        Self {
            typ: Default::default(),
            start: Default::default(),
            end: Default::default(),
            size: Default::default(),
            #[cfg(feature = "parent-links")]
            parent: -1,
        }
    }
}

/// A JsmnParser is the parser state for the jsmn library.
#[repr(C)]
#[derive(Debug, Copy, Default)]
pub struct JsmnParser {
    pub pos: usize,
    pub toknext: usize,
    pub toksuper: isize,
}

impl JsmnParser {
    pub fn new() -> Self {
        let parser = JsmnParser {
            pos: 0,
            toknext: 0,
            toksuper: 0,
        };
        unsafe {
            raw::jsmn_init(transmute(&parser));
        }

        parser
    }
}

impl Clone for JsmnParser {
    fn clone(&self) -> Self {
        *self
    }
}

/// This function is the core parsing function. It wraps the underlying
/// jsmn_parse function in a more Rustic interface by taking a slice
/// of JsmnTokens, and returning a Result instead of using sentinal values.
///
///
/// Simply provide a JsmnParser, a string, and a slice of JsmnTokens,
/// and the tokens will point to the locations of each JSON object within the
/// string.
///
/// If the function succeeds, it will return a usize giving how many
/// tokens were parsed, and on error it will return an JsmnErr describing the
/// problem encountered while parsing.
pub fn jsmn_parse(
    parser: &mut JsmnParser,
    js: &str,
    tokens: &mut [JsmnTok],
) -> Result<usize, JsmnErr> {
    unsafe fn cast_slice_mut<T, U>(src: &mut [T]) -> &mut [U] {
        assert_eq!(size_of::<T>(), size_of::<U>(), "Size mismatch");
        assert_eq!(align_of::<T>(), align_of::<U>(), "Alignment mismatch");

        let len = src.len();
        let ptr = src.as_mut_ptr() as *mut U;
        unsafe { std::slice::from_raw_parts_mut(ptr, len) }
    }

    let result: i32;
    unsafe {
        let raw_tokens: &mut [raw::jsmntok_t] = cast_slice_mut(tokens);

        result = raw::jsmn_parse(
            parser as *mut _ as *mut raw::jsmn_parser,
            js.as_ptr() as *const _,
            js.len(),
            raw_tokens.as_mut_ptr(),
            raw_tokens.len() as u32,
        );
    }

    if result < 0 {
        return match result {
            -1 => Err(JsmnErr::JsmErrorNoMem),
            -2 => Err(JsmnErr::JsmErrorInval),
            -3 => Err(JsmnErr::JsmErrorPart),
            _ => unreachable!(),
        };
    }

    Ok(result as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! parse {
        ($buf: expr, $len: expr) => {{
            let mut tokens = [JsmnTok::default(); $len];
            let mut parser = JsmnParser::new();

            jsmn_parse(&mut parser, $buf, &mut tokens).map(|parsed| {
                assert_eq!($len, parsed);
                tokens
            })
        }};
    }

    #[test]
    fn parse_empty() {
        let tokens = parse!(r#"{}"#, 1).unwrap();
        assert_eq!(
            &[JsmnTok {
                typ: JsmnType::JsmnObject,
                start: 0,
                end: 2,
                size: 0,
                ..Default::default()
            }],
            &tokens
        );

        let tokens = parse!(r#"[]"#, 1).unwrap();
        assert_eq!(
            &[JsmnTok {
                typ: JsmnType::JsmnArray,
                start: 0,
                end: 2,
                size: 0,
                ..Default::default()
            }],
            &tokens
        );

        let tokens = parse!(r#"[{},{}]"#, 3).unwrap();
        assert_eq!(
            &[
                JsmnTok {
                    typ: JsmnType::JsmnArray,
                    start: 0,
                    end: 7,
                    size: 2,
                    ..Default::default()
                },
                JsmnTok {
                    typ: JsmnType::JsmnObject,
                    start: 1,
                    end: 3,
                    size: 0,
                    #[cfg(feature = "parent-links")]
                    parent: 0,
                    ..Default::default()
                },
                JsmnTok {
                    typ: JsmnType::JsmnObject,
                    start: 4,
                    end: 6,
                    size: 0,
                    #[cfg(feature = "parent-links")]
                    parent: 0,
                    ..Default::default()
                }
            ],
            &tokens
        );
    }

    #[test]
    fn parse_object_mixed() {
        let tokens = parse!(r#"{"a":0}"#, 3).unwrap();
        assert_eq!(
            &[
                JsmnTok {
                    typ: JsmnType::JsmnObject,
                    start: 0,
                    end: 7,
                    size: 1,
                    ..Default::default()
                },
                JsmnTok {
                    typ: JsmnType::JsmnString,
                    start: 2,
                    end: 3,
                    size: 1,
                    #[cfg(feature = "parent-links")]
                    parent: 0,
                    ..Default::default()
                },
                JsmnTok {
                    typ: JsmnType::JsmnPrimitive,
                    start: 5,
                    end: 6,
                    size: 0,
                    #[cfg(feature = "parent-links")]
                    parent: 1,
                    ..Default::default()
                }
            ],
            &tokens
        );

        let tokens = parse!(r#"{"a":[]}"#, 3).unwrap();
        assert_eq!(
            &[
                JsmnTok {
                    typ: JsmnType::JsmnObject,
                    start: 0,
                    end: 8,
                    size: 1,
                    ..Default::default()
                },
                JsmnTok {
                    typ: JsmnType::JsmnString,
                    start: 2,
                    end: 3,
                    size: 1,
                    #[cfg(feature = "parent-links")]
                    parent: 0,
                    ..Default::default()
                },
                JsmnTok {
                    typ: JsmnType::JsmnArray,
                    start: 5,
                    end: 7,
                    size: 0,
                    #[cfg(feature = "parent-links")]
                    parent: 1,
                    ..Default::default()
                }
            ],
            &tokens
        );
    }

    #[test]
    fn parse_array_single_primitive() {
        let tokens = parse!(r#"[10]"#, 2).unwrap();
        assert_eq!(
            &[
                JsmnTok {
                    typ: JsmnType::JsmnArray,
                    start: 0,
                    end: 4,
                    size: 1,
                    ..Default::default()
                },
                JsmnTok {
                    typ: JsmnType::JsmnPrimitive,
                    start: 1,
                    end: 3,
                    size: 0,
                    #[cfg(feature = "parent-links")]
                    parent: 0,
                    ..Default::default()
                }
            ],
            &tokens
        );
    }

    #[test]
    fn parse_primitive_values() {
        assert_eq!(
            parse!(r#"{"boolVar": true}"#, 3).unwrap()[2],
            JsmnTok {
                typ: JsmnType::JsmnPrimitive,
                start: 12,
                end: 16,
                #[cfg(feature = "parent-links")]
                parent: 1,
                ..Default::default()
            },
        );
        assert_eq!(
            parse!(r#"{"boolVar": false}"#, 3).unwrap()[2],
            JsmnTok {
                typ: JsmnType::JsmnPrimitive,
                start: 12,
                end: 17,
                #[cfg(feature = "parent-links")]
                parent: 1,
                ..Default::default()
            },
        );
        assert_eq!(
            parse!(r#"{"nullVar": null}"#, 3).unwrap()[2],
            JsmnTok {
                typ: JsmnType::JsmnPrimitive,
                start: 12,
                end: 16,
                #[cfg(feature = "parent-links")]
                parent: 1,
                ..Default::default()
            },
        );
        assert_eq!(
            parse!(r#"{"intVar": 12}"#, 3).unwrap()[2],
            JsmnTok {
                typ: JsmnType::JsmnPrimitive,
                start: 11,
                end: 13,
                #[cfg(feature = "parent-links")]
                parent: 1,
                ..Default::default()
            },
        );
        assert_eq!(
            parse!(r#"{"floatVar": 12.345}"#, 3).unwrap()[2],
            JsmnTok {
                typ: JsmnType::JsmnPrimitive,
                start: 13,
                end: 19,
                #[cfg(feature = "parent-links")]
                parent: 1,
                ..Default::default()
            },
        );
    }

    #[test]
    fn parse_invalid_escaped_unicode() {
        assert_eq!(
            parse!(r#"{"a":"str\uFFGFstr"}"#, 3).unwrap_err(),
            JsmnErr::JsmErrorInval
        );
        assert_eq!(
            parse!(r#"{"a":"str\u@FfF"}"#, 3).unwrap_err(),
            JsmnErr::JsmErrorInval
        );
        assert_eq!(
            parse!(r#"{{"a":["\u028"]}"#, 4).unwrap_err(),
            JsmnErr::JsmErrorInval
        );
    }
}
