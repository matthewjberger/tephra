pub trait App {
    fn initialize(&mut self) {}
    fn update(&mut self) {}
    fn render(&mut self) {}
    fn cleanup(&mut self) {}
    fn on_resize(&mut self) {}
    fn on_key(&mut self) {}
    fn run(&mut self) {
        self.initialize();

        self.update();
        self.render();

        self.cleanup();
    }
}
