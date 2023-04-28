use crate::pixel::Pixel;

pub trait EmbedderCallbacks {
    fn log(&self, tag: String, message: String);

    fn draw(&mut self, width: usize, height: usize, buffer: Vec<Pixel>);

    // TODO(jiahaog): This shouldn't be here.
    fn draw_text(&mut self, x: usize, y: usize, text: &str);
}
