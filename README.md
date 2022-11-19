  ------------------------------------------------------------
< To start sending messages, you first need to know your friend multiaddr. Look for a log that starts with "/ip4/192..." and send to your friend.
1. Alice - listen for events: RUST_LOG=info cargo run
2. Bob - dial Bob multiaddr: RUST_LOG=info cargo run -- --peer /ip4/x.x.x.x/tcp/xxxxx
Now they are connected and can start sending messages on the terminal. >
  ------------------------------------------------------------
    \   ^__^
     \  (oo)\______
        (__)\      )\/\
           ||----w |
           ||     ||
