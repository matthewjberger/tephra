use support::app::App;

#[derive(Default)]
struct DemoApp;

impl App for DemoApp {}

fn main() {
    let mut demo_app = DemoApp::default();
    demo_app.run();
}
