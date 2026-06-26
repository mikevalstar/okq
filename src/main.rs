//! okq binary entry point — a thin shell over the library's [`okq::run`].

fn main() {
    std::process::exit(okq::run());
}
