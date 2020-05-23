use shader_compilation::compile_shaders;

fn main() {
    let shader_directory = "assets/shaders";
    let shader_glob = shader_directory.to_owned() + "/**/*.glsl";
    if compile_shaders(&shader_glob).is_err() {
        println!("Failed to recompile shaders!");
    }
}