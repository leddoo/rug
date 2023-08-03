pub mod svg {
    pub const CAR: &str = include_str!("../res/car.svg");
    pub const GALLARDO: &str = include_str!("../res/gallardo-simp.svg");
    pub const GRADIENT_TRI: &str = include_str!("../res/gradient-tri.svg");
    pub const INTERTWINGLY: &str = include_str!("../res/intertwingly.svg");
    pub const PARIS: &str = include_str!("../res/paris-30k.svg");
    pub const RADIAL_GRADIENT_1: &str = include_str!("../res/radialgradient1.svg");
    pub const SCIMITAR: &str = include_str!("../res/scimitar-simp.svg");
    pub const TIGER: &str = include_str!("../res/tiger.svg");
    pub const TOMMEK_CAR: &str = include_str!("../res/tommek_Car-simp.svg");
}


use rug::{cmd::*, color::*, geometry::Transform};
use xmlparser::*;
use core::str::FromStr;
use std::collections::HashMap;


pub fn parse_svg(svg: &str) -> CmdBuf {
    spall::trace_scope!("vg-inputs::parse_svg");

    CmdBuf::new(|cb| {
        let mut parser = SvgParser {
            toker: Tokenizer::from(svg),
            cb,
            defs: HashMap::new(),
        };
        parser.parse_svg();
    })
}


enum Def {
    LinearGradient(LinearGradientId),
}

enum Paint {
    Solid(u32),
    LinearGradient(LinearGradientId),
}

struct SvgParser<'a, 'cb> {
    toker: Tokenizer<'a>,
    cb: &'a mut CmdBufBuilder<'cb>,
    defs: HashMap<&'a str, Def>,
}

impl<'a, 'cb> SvgParser<'a, 'cb> {
    fn parse_svg(&mut self) {
        spall::trace_scope!("vg-inputs::parse_svg::svg");

        // find `<svg>`
        loop {
            let at = self.toker.next().unwrap().unwrap();
            match at {
                Token::ElementStart { prefix, local, span: _ } => {
                    assert!(prefix.is_empty());
                    assert_eq!(local.as_str(), "svg");
                    break;
                }

                // ignored.
                Token::Declaration {..} |
                Token::ProcessingInstruction {..} |
                Token::Comment {..} |
                Token::EntityDeclaration {..} |
                Token::Text {..} |
                Token::Cdata {..} => (),

                // not sure.
                Token::DtdStart {..} |
                Token::EmptyDtd {..} |
                Token::DtdEnd {..} => unimplemented!(),

                // errors.
                Token::Attribute {..} |
                Token::ElementEnd {..} => unimplemented!(),
            }
        }

        if self.skip_attrs() {
            self.visit_children(|this, at| this.parse_element(at));
        }
    }

    fn parse_element(&mut self, at: &Token) {
        spall::trace_scope!("vg-inputs::parse_svg::element");

        let kind = match at {
            Token::ElementStart { prefix, local, span: _ } => {
                if !prefix.is_empty() {
                    println!("unknown element \"{}:{}\"", prefix, local);
                    if self.skip_attrs() {
                        self.skip_children();
                    }
                    return;
                }

                local.as_str()
            }

            // ignored.
            Token::Declaration {..} |
            Token::ProcessingInstruction {..} |
            Token::Comment {..} |
            Token::EntityDeclaration {..} |
            Token::Text {..} |
            Token::Cdata {..} => return,

            // not sure.
            Token::DtdStart {..} |
            Token::EmptyDtd {..} |
            Token::DtdEnd {..} => unimplemented!(),

            // errors.
            Token::Attribute {..} |
            Token::ElementEnd {..} => unimplemented!(),
        };

        match kind {
            "defs" => {
                if self.skip_attrs() {
                    self.visit_children(|this, at| this.parse_def(at));
                }
            }

            "g" => {
                if self.skip_attrs() {
                    self.visit_children(|this, at| this.parse_element(at));
                }
            }

            "path" => {
                spall::trace_scope!("vg-inputs::parse_svg::path");

                let mut path:           Option<rug::path::Path> = None;
                let mut fill:           Option<Paint> = None;
                let mut fill_opacity:   Option<f32> = None;
                let mut stroke:         Option<Paint> = None;
                let mut stroke_width:   Option<f32> = None;
                let mut stroke_opacity: Option<f32> = None;

                loop {
                    match self.toker.next().unwrap().unwrap() {
                        Token::Attribute { prefix, local, value, .. } => {
                            if !prefix.is_empty() { continue; }

                            path = Some(self.cb.build_path(|pb| {
                                match local.as_str() {
                                    "d" => {
                                        let mut error = false;

                                        for e in svgtypes::PathParser::from(&*value) {
                                            use svgtypes::PathSegment::*;
                                            match e.unwrap() {
                                                MoveTo { abs, x, y } => {
                                                    if abs {
                                                        pb.move_to([x as f32, y as f32].into());
                                                    }
                                                    else {
                                                        println!("abs not implemented");
                                                        error = true;
                                                    }
                                                }

                                                LineTo { abs, x, y } => {
                                                    if abs {
                                                        pb.line_to([x as f32, y as f32].into());
                                                    }
                                                    else {
                                                        println!("abs not implemented");
                                                        error = true;
                                                    }
                                                }

                                                Quadratic { abs, x1, y1, x, y } => {
                                                    if abs {
                                                        pb.quad_to([x1 as f32, y1 as f32].into(), [x as f32, y as f32].into());
                                                    }
                                                    else {
                                                        println!("abs not implemented");
                                                        error = true;
                                                    }
                                                }

                                                CurveTo { abs, x1, y1, x2, y2, x, y } => {
                                                    if abs {
                                                        pb.cubic_to([x1 as f32, y1 as f32].into(), [x2 as f32, y2 as f32].into(), [x as f32, y as f32].into());
                                                    }
                                                    else {
                                                        println!("abs not implemented");
                                                        error = true;
                                                    }
                                                }

                                                ClosePath { abs: _ } => {
                                                    pb.close_path();
                                                }

                                                e => {
                                                    println!("unsupported path cmd {:?}", e);
                                                    error = true;
                                                }
                                            }
                                        }

                                        if error {
                                            pb.clear();
                                        }
                                    }

                                    "fill" => {
                                        let paint = svgtypes::Paint::from_str(&*value).unwrap();
                                        match paint {
                                            svgtypes::Paint::None => (),

                                            svgtypes::Paint::Color(c) => {
                                                fill = Some(Paint::Solid(argb_pack_u8s(c.red, c.green, c.blue, c.alpha)));
                                            }

                                            svgtypes::Paint::FuncIRI(uri, _) => {
                                                if let Some(def) = self.defs.get(uri) {
                                                    match *def {
                                                        Def::LinearGradient(g) => {
                                                            fill = Some(Paint::LinearGradient(g));
                                                        }
                                                    }
                                                }
                                                else {
                                                    println!("unknown fill {:?}", uri);
                                                }
                                            }

                                            _ => {
                                                println!("unknown paint {:?}", paint);
                                            }
                                        }
                                    }

                                    "fill-opacity" => {
                                        fill_opacity = Some(svgtypes::Number::from_str(&*value).unwrap().0 as f32);
                                    }

                                    "stroke" => {
                                        let paint = svgtypes::Paint::from_str(&*value).unwrap();
                                        match paint {
                                            svgtypes::Paint::None => (),

                                            svgtypes::Paint::Color(c) => {
                                                stroke = Some(Paint::Solid(argb_pack_u8s(c.red, c.green, c.blue, c.alpha)));
                                            }

                                            svgtypes::Paint::FuncIRI(uri, fallback) => {
                                                println!("todo: stroke uri {:?}/{:?}", uri, fallback);
                                            }

                                            _ => {
                                                println!("unknown paint {:?}", paint);
                                            }
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

                if let Some(paint) = fill {
                    let opacity = fill_opacity.unwrap_or(1.0);
                    match paint {
                        Paint::Solid(color) => {
                            let [r, g, b, a] = *argb_unpack(color);
                            let color = argb_pack([r, g, b, a*opacity].into());
                            self.cb.push(Cmd::FillPathSolid { path, color });
                        }

                        Paint::LinearGradient(gradient) => {
                            self.cb.push(Cmd::FillPathLinearGradient { path, gradient, opacity });
                        }
                    }
                }

                if let Some(paint) = stroke {
                    let opacity = stroke_opacity.unwrap_or(1.0);
                    let width = stroke_width.unwrap_or(1.0);
                    match paint {
                        Paint::Solid(color) => {
                            let [r, g, b, a] = *argb_unpack(color);
                            let color = argb_pack([r, g, b, a*opacity].into());
                            self.cb.push(Cmd::StrokePathSolid { path, color, width });
                        }

                        Paint::LinearGradient(_) => {
                            unimplemented!()
                        }
                    }
                }
            }

            _ => {
                println!("unsupported element: {:?}", kind);
                if self.skip_attrs() {
                    self.skip_children();
                }
            }
        }
    }

    fn parse_def(&mut self, at: &Token) {
        spall::trace_scope!("vg-inputs::parse_svg::def");

        let kind = match at {
            Token::ElementStart { prefix, local, span: _ } => {
                if !prefix.is_empty() {
                    println!("unknown def \"{}:{}\"", prefix, local);
                    if self.skip_attrs() {
                        self.skip_children();
                    }
                    return;
                }

                local.as_str()
            }

            // ignored.
            Token::Declaration {..} |
            Token::ProcessingInstruction {..} |
            Token::Comment {..} |
            Token::EntityDeclaration {..} |
            Token::Text {..} |
            Token::Cdata {..} => return,

            // not sure.
            Token::DtdStart {..} |
            Token::EmptyDtd {..} |
            Token::DtdEnd {..} => unimplemented!(),

            // errors.
            Token::Attribute {..} |
            Token::ElementEnd {..} => unimplemented!(),
        };

        match kind {
            "linearGradient" => {
                let mut id: Option<&'a str> = None;
                let mut x0 = 0.0;
                let mut y0 = 0.0;
                let mut x1 = 1.0;
                let mut y1 = 1.0;
                let mut spread = SpreadMethod::Pad;
                let mut units = GradientUnits::Relative;
                let mut tfx = Transform::ID;

                let children = loop {
                    match self.toker.next().unwrap().unwrap() {
                        Token::Attribute { prefix, local, value, .. } => {
                            if !prefix.is_empty() { continue; }

                            match &*local {
                                "id" => {
                                    id = Some(value.as_str());
                                }

                                "x1" => x0 = svgtypes::Number::from_str(&*value).unwrap().0 as f32,
                                "y1" => y0 = svgtypes::Number::from_str(&*value).unwrap().0 as f32,
                                "x2" => x1 = svgtypes::Number::from_str(&*value).unwrap().0 as f32,
                                "y2" => y1 = svgtypes::Number::from_str(&*value).unwrap().0 as f32,

                                "spreadMethod" => {
                                    match value.as_str() {
                                        "pad"     => spread = SpreadMethod::Pad,
                                        "reflect" => spread = SpreadMethod::Reflect,
                                        "repeat"  => spread = SpreadMethod::Repeat,
                                        _ => unreachable!()
                                    }
                                }

                                "gradientUnits" => {
                                    match value.as_str() {
                                        "userSpaceOnUse"    => units = GradientUnits::Absolute,
                                        "objectBoundingBox" => units = GradientUnits::Relative,
                                        "reset" => (),
                                        _ => unreachable!()
                                    }
                                }

                                "gradientTransform" => {
                                    if value.as_str() != "reset" {
                                        let t = svgtypes::Transform::from_str(&*value).unwrap();
                                        tfx = Transform { columns: [
                                            [t.a as f32, t.b as f32].into(),
                                            [t.c as f32, t.d as f32].into(),
                                            [t.e as f32, t.f as f32].into(),
                                        ]};
                                    }
                                }

                                _ => {
                                    println!("unknown linear gradient attr {:?}", local);
                                }
                            }
                        }

                        Token::ElementEnd { end: ElementEnd::Empty, span: _ } => break false,
                        Token::ElementEnd { end: ElementEnd::Open, span: _ } => break true,

                        _ => unimplemented!()
                    }
                };

                if children {
                    let stops = self.parse_gradient_stops();

                    if let Some(name) = id {
                        let id = self.cb.push_linear_gradient(LinearGradient {
                            p0: [x0, y0].into(),
                            p1: [x1, y1].into(),
                            spread,
                            units,
                            tfx,
                            stops,
                        });
                        self.defs.insert(name, Def::LinearGradient(id));
                    }
                }
            }

            _ => {
                println!("unknown def {:?}", kind);
                if self.skip_attrs() {
                    self.skip_children();
                }
            }
        }
    }

    fn parse_gradient_stops(&mut self) -> &'cb [GradientStop] {
        self.cb.build_gradient_stops(|sb| {
            loop {
                let at = self.toker.next().unwrap().unwrap();
                match at {
                    Token::ElementStart { prefix, local, span: _ } => {
                        assert!(prefix.is_empty());
                        assert_eq!(local.as_str(), "stop");
                    }

                    Token::ElementEnd { end: ElementEnd::Close(_, _), span: _ } => return,

                    // ignored.
                    Token::Declaration {..} |
                    Token::ProcessingInstruction {..} |
                    Token::Comment {..} |
                    Token::EntityDeclaration {..} |
                    Token::Text {..} |
                    Token::Cdata {..} => continue,

                    // not sure.
                    Token::DtdStart {..} |
                    Token::EmptyDtd {..} |
                    Token::DtdEnd {..} => unimplemented!(),

                    // errors.
                    Token::Attribute {..} |
                    Token::ElementEnd {..} => unimplemented!()
                }


                let mut offset = 0.0;
                let mut opacity = 1.0;
                let mut color = argb_pack_u8s(0, 0, 0, 255);

                loop {
                    match self.toker.next().unwrap().unwrap() {
                        Token::Attribute { prefix, local, value, .. } => {
                            if !prefix.is_empty() { continue; }

                            match &*local {
                                "offset" => offset = svgtypes::Number::from_str(&*value).unwrap().0 as f32,

                                "stop-opacity" => opacity = svgtypes::Number::from_str(&*value).unwrap().0 as f32,

                                "stop-color" => {
                                    let c = svgtypes::Color::from_str(&*value).unwrap();
                                    color = argb_pack_u8s(c.red, c.green, c.blue, c.alpha);
                                }

                                _ => {
                                    println!("unknown gradient stop attr {:?}", local);
                                }
                            }
                        }

                        Token::ElementEnd { end: ElementEnd::Empty, span: _ } => break,

                        _ => unimplemented!()
                    }
                }

                let [r, g, b, a] = *argb_unpack(color);
                color = argb_pack([r, g, b, a*opacity].into());

                sb.push(GradientStop { offset, color });
            }
        })
    }


    #[must_use]
    fn skip_attrs(&mut self) -> bool {
        loop {
            match self.toker.next().unwrap().unwrap() {
                Token::Text {..} |
                Token::Attribute {..} => (),
                Token::ElementEnd { end: ElementEnd::Open, span: _ } => return true,
                Token::ElementEnd { end: ElementEnd::Empty, span: _ } => return false,
                t => {
                    dbg!(t);
                    unimplemented!()
                }
            }
        }
    }

    fn visit_children<F: Fn(&mut SvgParser, &Token)>(&mut self, f: F) {
        loop {
            let at = self.toker.next().unwrap().unwrap();
            if let Token::ElementEnd { end: ElementEnd::Close(_, _), span: _ } = at {
                break;
            }
            f(self, &at);
        }
    }

    fn skip_children(&mut self) {
        let mut depth = 0;
        loop {
            let at = self.toker.next().unwrap().unwrap();
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
                            depth -= 1;
                        }

                        ElementEnd::Empty => {
                            depth -= 1;
                        }
                    }
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
}

