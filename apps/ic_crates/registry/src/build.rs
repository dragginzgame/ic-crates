//! Generate the accepted database schema and this component's Canic role authority.

mod design;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    canic::build!("../canic.toml");
    println!("cargo:rerun-if-changed=src/design");
    icydb::build::build_canister!(design::RegistryCanister)
}
