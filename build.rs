use windres::Build;

fn main() {
    Build::new().compile("main.rc").unwrap();
}