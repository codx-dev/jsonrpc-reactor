# JSON-RPC

JSON-RPC 2.0 Rust implementation with tokio backend.

Things are meant to be simple.

Example:

```rust
use std::time::Duration;

use jsonrpc_reactor::*;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    let capacity = 100;

    let (rtx, mut requests) = mpsc::channel(capacity);
    let (ntx, mut notifications) = mpsc::channel(capacity);

    let (mut reactor, service) = Reactor::spawn(capacity, rtx, ntx);

    // requests handler
    tokio::spawn(async move {
        while let Some(r) = requests.recv().await {
            match r.method.as_str() {
                "math/inc" => {
                    let number = r.params.as_object().expect("name should be object")["number"]
                        .as_i64()
                        .expect("the provided argument isn't a valid number");

                    let number = number.wrapping_add(1).into();

                    let response = Response {
                        id: r.id,
                        result: Ok(number),
                    };

                    service.send(response).await.ok();
                }

                _ => println!("invalid method"),
            }
        }
    });

    // notifications handler
    tokio::spawn(async move {
        while let Some(n) = notifications.recv().await {
            match n.method.as_str() {
                "misc/greet" => {
                    println!(
                        "Hello, {}!",
                        n.params.as_object().expect("name should be object")["name"]
                    )
                }
                _ => println!("invalid method"),
            }
        }
    });

    let method = "misc/greet";
    let timeout = Some(Duration::from_secs(2));
    let params: Params = json!({
        "name": "Victor"
    })
    .try_into()
    .expect("failed to create params");

    reactor.notify(method, params, timeout).await;

    let method = "math/inc";
    let timeout = Some(Duration::from_secs(2));
    let params: Params = json!({
        "number": 15
    })
    .try_into()
    .expect("failed to create params");

    let mut awaiting = reactor
        .request(method, params, timeout)
        .await
        .expect("failed to fetch oneshot receiver");

    // oneshot is blocking and shouldn't be executed in the same context of the main reactor.
    //
    // in production, requester and responder won't be in the same context
    loop {
        if let Some(response) = awaiting.try_recv().ok() {
            println!("response: {:?}", response);

            break;
        }
    }
}
```
