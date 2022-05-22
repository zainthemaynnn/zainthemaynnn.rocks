use actix::prelude::*;
use image::{codecs::png::PngEncoder, ImageEncoder, Rgba, RgbaImage};
use imageproc::{drawing, point::Point, rect::Rect};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    mem::size_of,
    sync::{Arc, RwLock},
};

const WIDTH: usize = 1920;
const HEIGHT: usize = 1080;
const PX_SIZE: usize = size_of::<Rgba<u8>>();
const IMG_SIZE: usize = WIDTH * HEIGHT * PX_SIZE;
const DEFAULT_CANVAS_COLOR: Rgba<u8> = Rgba([248u8, 248u8, 255u8, 0u8]);

type Client = Recipient<Draw>;

#[derive(Clone, Debug, Message)]
#[rtype(result = "Vec<u8>")]
pub struct Connect {
    pub client: Client,
}

#[derive(Clone, Debug, Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub client: Client,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CssColor {
    Black,
    White,
    Red,
    Lime,
    Blue,
    Yellow,
    Magenta,
    Cyan,
}

impl CssColor {
    fn as_rgba(&self) -> Rgba<u8> {
        match self {
            Self::Black => Rgba([0u8, 0u8, 0u8, 255u8]),
            Self::White => Rgba([255u8, 255u8, 255u8, 255u8]),
            Self::Red => Rgba([255u8, 0u8, 0u8, 255u8]),
            Self::Lime => Rgba([0u8, 255u8, 0u8, 255u8]),
            Self::Blue => Rgba([0u8, 0u8, 255u8, 255u8]),
            Self::Yellow => Rgba([255u8, 255u8, 0u8, 255u8]),
            Self::Magenta => Rgba([255u8, 0u8, 255u8, 255u8]),
            Self::Cyan => Rgba([0u8, 255u8, 255u8, 255u8]),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Message)]
#[rtype(result = "()")]
pub struct Draw {
    pub x0: f32,
    pub x1: f32,
    pub y0: f32,
    pub y1: f32,
    pub color: CssColor,
    pub thickness: f32,
}

#[derive(Clone, Debug, Default)]
pub struct Server {
    pub clients: HashSet<Client>,
    pub img: Arc<RwLock<RgbaImage>>,
}

impl Server {
    pub fn new() -> Self {
        let mut img = RgbaImage::new(WIDTH as u32, HEIGHT as u32);
        drawing::draw_filled_rect_mut(
            &mut img,
            Rect::at(0, 0).of_size(WIDTH as u32, HEIGHT as u32),
            DEFAULT_CANVAS_COLOR,
        );
        Self {
            clients: HashSet::new(),
            img: Arc::new(RwLock::new(img)),
        }
    }
}

impl Actor for Server {
    type Context = Context<Self>;
}

impl Handler<Connect> for Server {
    type Result = Vec<u8>;

    fn handle(&mut self, msg: Connect, _ctx: &mut Self::Context) -> Self::Result {
        self.clients.insert(msg.client);

        let mut buf = String::from("data:image/png;base64,");
        if let Ok(img) = self.img.clone().read() {
            let mut data = Vec::new();
            let encoder = PngEncoder::new(&mut data);
            encoder
                .write_image(
                    &img.as_raw(),
                    img.width(),
                    img.height(),
                    image::ColorType::Rgba8,
                )
                .expect("failed rgba to png conversion");
            buf.reserve(IMG_SIZE);
            base64::encode_config_buf(&data, base64::STANDARD, &mut buf);
        }
        buf.as_bytes().to_vec()
    }
}

impl Handler<Disconnect> for Server {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _ctx: &mut Self::Context) -> Self::Result {
        self.clients.remove(&msg.client);
    }
}

impl Handler<Draw> for Server {
    type Result = ();

    fn handle(&mut self, msg: Draw, _ctx: &mut Self::Context) -> Self::Result {
        if let Ok(mut img) = self.img.clone().write() {
            // mhf4u7 to the rescue?
            let m = -(msg.x1 - msg.x0) / (msg.y1 - msg.y0);
            let a = f32::atan(m);
            let dx = msg.thickness / 2. * f32::cos(a);
            let dy = msg.thickness / 2. * f32::sin(a);
            let points = &[
                Point::new((msg.x0 + dx) as i32, (msg.y0 + dy) as i32),
                Point::new((msg.x1 + dx) as i32, (msg.y1 + dy) as i32),
                Point::new((msg.x1 - dx) as i32, (msg.y1 - dy) as i32),
                Point::new((msg.x0 - dx) as i32, (msg.y0 - dy) as i32),
            ];

            // for some reason, this happens naturally sometimes
            // not sure why, but might as well get the data sanitized anyways
            if points[0] != points[points.len() - 1] {
                drawing::draw_polygon_mut(&mut *img, points, msg.color.as_rgba());
            }
        }

        for client in &self.clients {
            client.do_send(msg.clone());
        }
    }
}
