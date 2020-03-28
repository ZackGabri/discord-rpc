#[cfg(windows)] extern crate windres;

fn main() {
	#[cfg(windows)] {
		use windres::Build;
		Build::new().compile("resources.rc").unwrap();
	}
}