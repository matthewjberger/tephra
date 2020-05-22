use support::app::{run_app, App};

#[derive(Default)]
struct DemoApp;

impl App for DemoApp {}

fn main() {
    run_app(DemoApp::default(), "Triangle");
}
