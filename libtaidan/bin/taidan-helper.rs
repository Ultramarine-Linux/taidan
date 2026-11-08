//! `/usr/libexec/taidan-helper`, the one program the OOBE runs as root, see libtaidan's `helper`
//! module
fn main() {
    std::process::exit(libtaidan::helper::main());
}
