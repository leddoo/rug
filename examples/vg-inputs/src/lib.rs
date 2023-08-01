mod tiger;
pub use tiger::*;


pub const PARIS_SVG: &str = include_str!("../res/paris-30k.svg");


use rug::{cmd::*, color::*};
use xmlparser::*;

pub fn parse_svg(svg: &str) -> CmdBuf {
    spall::trace_scope!("vg-inputs::parse_svg");

    fn parse(cb: &mut CmdBufBuilder, toker: &mut Tokenizer, at: &Token) -> bool {
        let kind = match at {
            Token::ElementStart { prefix, local, span: _ } => {
                assert!(prefix.is_empty());
                local.as_str()
            }

            // ignored.
            Token::Declaration {..} |
            Token::ProcessingInstruction {..} |
            Token::Comment {..} |
            Token::EntityDeclaration {..} |
            Token::Text {..} |
            Token::Cdata {..} => return true,

            // not sure.
            Token::DtdStart {..} |
            Token::EmptyDtd {..} |
            Token::DtdEnd {..} => unimplemented!(),

            // errors.
            Token::Attribute {..} |
            Token::ElementEnd {..} => unimplemented!(),
        };

        fn skip_attrs(toker: &mut Tokenizer) {
            loop {
                match toker.next().unwrap().unwrap() {
                    Token::Attribute {..} => (),
                    Token::ElementEnd { end: ElementEnd::Open, span: _ } => break,
                    _ => unimplemented!()
                }
            }
        }

        fn visit_children(cb: &mut CmdBufBuilder, toker: &mut Tokenizer) {
            loop {
                let at = toker.next().unwrap().unwrap();
                if let Token::ElementEnd { end: ElementEnd::Close(_, _), span: _ } = at {
                    break;
                }
                parse(cb, toker, &at);
            }
        }

        fn skip_children(toker: &mut Tokenizer) {
            let mut depth = 0;
            loop {
                let at = toker.next().unwrap().unwrap();
                match at {
                    Token::ElementStart {..} => {
                        depth += 1;
                    }

                    Token::ElementEnd { end, .. } => {
                        match end {
                            ElementEnd::Open => {
                                assert!(depth > 0);
                            }

                            ElementEnd::Close(_, _) => {
                                if depth == 0 {
                                    break;
                                }
                            }

                            ElementEnd::Empty => {}
                        }

                        depth -= 1;
                    }

                    Token::Declaration {..} |
                    Token::ProcessingInstruction {..} |
                    Token::Comment {..} |
                    Token::EntityDeclaration {..} |
                    Token::Attribute {..} |
                    Token::Text {..} |
                    Token::Cdata {..} => (),

                    Token::DtdStart {..} |
                    Token::EmptyDtd {..} |
                    Token::DtdEnd {..} => unimplemented!()
                }
            }
        }

        match kind {
            "svg" => {
                spall::trace_scope!("vg-inputs::parse_svg::svg");
                skip_attrs(toker);
                visit_children(cb, toker);
                return false;
            }

            "defs" => {
                skip_attrs(toker);
                skip_children(toker);
            }

            "g" => {
                spall::trace_scope!("vg-inputs::parse_svg::g");
                skip_attrs(toker);
                visit_children(cb, toker);
            }

            "path" => {
                spall::trace_scope!("vg-inputs::parse_svg::path");

                let mut path:           Option<rug::path::Path> = None;
                let mut fill:           Option<svgtypes::Color> = None;
                let mut fill_opacity:   Option<f32> = None;
                let mut stroke:         Option<svgtypes::Color> = None;
                let mut stroke_width:   Option<f32> = None;
                let mut stroke_opacity: Option<f32> = None;

                loop {
                    match toker.next().unwrap().unwrap() {
                        Token::Attribute { prefix, local, value, .. } => {
                            if !prefix.is_empty() { continue; }

                            use core::str::FromStr;

                            path = Some(cb.build_path(|pb| {
                                match local.as_str() {
                                    "d" => {
                                        for e in svgtypes::PathParser::from(&*value) {
                                            use svgtypes::PathSegment::*;
                                            match e.unwrap() {
                                                MoveTo { abs, x, y } => {
                                                    assert!(abs);
                                                    pb.move_to([x as f32, y as f32].into());
                                                }

                                                LineTo { abs, x, y } => {
                                                    assert!(abs);
                                                    pb.line_to([x as f32, y as f32].into());
                                                }

                                                Quadratic { abs, x1, y1, x, y } => {
                                                    assert!(abs);
                                                    pb.quad_to([x1 as f32, y1 as f32].into(), [x as f32, y as f32].into());
                                                }

                                                CurveTo { abs, x1, y1, x2, y2, x, y } => {
                                                    assert!(abs);
                                                    pb.cubic_to([x1 as f32, y1 as f32].into(), [x2 as f32, y2 as f32].into(), [x as f32, y as f32].into());
                                                }

                                                ClosePath { abs } => {
                                                    assert!(abs);
                                                    pb.close_path();
                                                }

                                                _ => unimplemented!()
                                            }
                                        }
                                    }

                                    "fill" => {
                                        if &*value != "none" {
                                            fill = Some(svgtypes::Color::from_str(&*value).unwrap());
                                        }
                                    }

                                    "fill-opacity" => {
                                        fill_opacity = Some(svgtypes::Number::from_str(&*value).unwrap().0 as f32);
                                    }

                                    "stroke" => {
                                        if &*value != "none" {
                                            stroke = Some(svgtypes::Color::from_str(&*value).unwrap());
                                        }
                                    }

                                    "stroke-width" => {
                                        stroke_width = Some(svgtypes::Number::from_str(&*value).unwrap().0 as f32);
                                    }

                                    "stroke-opacity" => {
                                        stroke_opacity = Some(svgtypes::Number::from_str(&*value).unwrap().0 as f32);
                                    }

                                    "id" => {
                                        //println!("{:?}", &*value);
                                    }

                                    _ => ()
                                }
                            }));

                        }

                        Token::ElementEnd { end: ElementEnd::Empty, span: _ } => break,

                        _ => unimplemented!()
                    }
                }

                let path = path.unwrap();

                if let Some(color) = fill {
                    let a = fill_opacity.unwrap_or(1.0);
                    let a = ((a * (color.alpha as f32 / 255.0)) * 255.0) as u8;
                    let color = argb_pack_u8s(color.red, color.green, color.blue, a);
                    cb.push(Cmd::FillPathSolid { path, color });
                }

                if let Some(color) = stroke {
                    let a = stroke_opacity.unwrap_or(1.0);
                    let a = ((a * (color.alpha as f32 / 255.0)) * 255.0) as u8;
                    let color = argb_pack_u8s(color.red, color.green, color.blue, a);
                    let width = stroke_width.unwrap_or(1.0);
                    cb.push(Cmd::StrokePathSolid { path, color, width });
                }
            }

            _ => {
                println!("unsupported element: {:?}", kind);
                skip_attrs(toker);
                skip_children(toker);
            }
        }

        return true;
    }

    CmdBuf::new(|cb| {
        let mut toker = Tokenizer::from(svg);
        loop {
            let at = toker.next().unwrap().unwrap();
            if !parse(cb, &mut toker, &at) {
                break;
            }
        }
        assert!(toker.next().is_none());
    })
}


