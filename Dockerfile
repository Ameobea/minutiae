FROM ekidd/rust-musl-builder

RUN ~/.cargo/bin/rustup default nightly-2018-03-15
RUN ~/.cargo/bin/rustup target add x86_64-unknown-linux-musl
RUN ~/.cargo/bin/rustup update

ADD ./colony-server /home/rust/src
ADD ./colony /home/rust/colony
ADD ./minutiae /home/rust/minutiae
RUN sudo chown -R rust:rust /home/rust

RUN cargo build --release --target x86_64-unknown-linux-musl

CMD /home/rust/src/target/x86_64-unknown-linux-musl/debug/colony-server
