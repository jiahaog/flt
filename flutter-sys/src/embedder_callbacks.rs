use crate::pixel::Pixel;

pub trait EmbedderCallbacks {
    fn log(&self, tag: String, message: String);

    fn draw(&mut self, width: usize, height: usize, buffer: Vec<Pixel>);
}
