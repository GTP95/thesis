# thesis
My master's thesis.

## registration-poc 
Contains a Rust cargo project consisting of a proof of concept implementing the generation of an IRMA attribute for a study participant.  
To generate the relative documentation, run `cargo doc` inside this directory. The resulting documentation will be inside `registration-poc/target/doc` and can be viewed using any web browser. Rustdoc has many options to generate documentation, you can read more in [the rustdoc book](https://doc.rust-lang.org/rustdoc).

## participant-login-poc
Contains a Rust proof of concept implementing the login flow for a PEP study participant.  
  
### Documentation
To generate the relative documentation, run `cargo doc` inside this directory. The resulting documentation will be inside `participant-login-poc/target/doc` and can be viewed using any web browser. Rustdoc has many options to generate documentation, you can read more in [the rustdoc book](https://doc.rust-lang.org/rustdoc).  
  
### Compiling and running
This project uses `cargo`. To compile and run a development build, run inside this directory `cargo run -p client` and `cargo run -p server`. To produce a release binary, use `cargo build --release`. You can read more about cargo inside [The Cargo Book](https://doc.rust-lang.org/stable/cargo). If you set the `RUST_LOG` environment variable to `debug`, both the client and the server will print debug information.