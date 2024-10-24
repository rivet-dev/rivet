use types_proto_build;

fn main() -> std::io::Result<()> {
	// Build schema
	types_proto_build::compile()
}
