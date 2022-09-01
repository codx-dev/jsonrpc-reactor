# JSON-RPC

JSON-RPC 2.0 Rust implementation with tokio backend.

# Example

```rust
use std::time::Duration;

use jsonrpc_reactor::*;

// a request handler is expected to take a request and generate a
// response.
//
// optimally, it will be infallible, returning valid RPC errors in
// case it fails. this behavior is a good practice, but it is not
// enforced by the library, leaving the option on how to implement
// that to the user
//
// it can be a complicated structure or a simple function, as in this
// example
async fn requests_handler(request: Request) -> Response {
    let Request { id, method, params } = request;

    match method.as_str() {
        "math/inc" => {
            let number = params
                .as_object()
                .expect("name should be object")["number"]
                .as_i64()
                .expect("the provided argument isn't a valid number");

            let number = number
                .wrapping_add(1)
                .into();

            Response {
                id: id,
                result: Ok(number),
            }
        }

        _ => Response {
            id: id,
            result: Err(RpcError {
                code: -1,
                message: "invalid method".into(),
                data: method.into(),
            }),
        },
    }
}

// a notification handler is expected to take a notification without a
// reply.
//
// it is the same as the request handle. using channels will give the
// user great flexibility so he can chose how to concretely implement
// the handler
async fn notifications_handler(notification: Notification) {
    let Notification { method, params } = notification;

    match method.as_str() {
        "misc/greet" => {
            let name = &params
                .as_object()
                .expect("name should be object")["name"];

            println!("Hello, {}!", name)
        }

        _ => println!("invalid method"),
    }
}

#[tokio::main]
async fn main() {
    // this is the buffer capacity so the internal maps and channels
    // will tweak around that
    let capacity = 100;

    // setup outbound requests and notifications channels
    let (rtx, mut requests) = mpsc::channel(capacity);
    let (ntx, mut notifications) = mpsc::channel(capacity);

    // spawn the reactor thread, returning its controller and the
    // channel that will submit requests responses to the reactor
    let (mut reactor, service) = Reactor::spawn(capacity, rtx, ntx);

    // requests handler thread. consume the service channel so the
    // handler can submit responses for the requests
    tokio::spawn(async move {
        while let Some(r) = requests.recv().await {
            let response = requests_handler(r).await;

            service.send(response).await.ok();
        }
    });

    // notifications handler thread
    tokio::spawn(async move {
        while let Some(n) = notifications.recv().await {
            notifications_handler(n).await;
        }
    });

    // the notifications timeout is used by the tokio channels in
    // case the handler can't take more notifications
    let method = "misc/greet";
    let timeout = Some(Duration::from_secs(2));
    let params: Params = json!({
        "name": "Victor"
    })
    .try_into()
    .expect("failed to create params");

    reactor.notify(method, params, timeout).await;

    // the timeout is used both by the tokio channels and reactor; the
    // latter discarding pending responses in case the handler takes
    // too much time to reply. the oneshot channel will receive a
    // valid JSONRPC response with a `timeout` error in case the
    // request is dropped
    let method = "math/inc";
    let timeout = Some(Duration::from_secs(2));
    let params: Params = json!({
        "number": 15
    })
    .try_into()
    .expect("failed to create params");

    // fetch the oneshot channel for this specific request
    let mut awaiting = reactor
        .request(method, params, timeout)
        .await
        .expect("failed to fetch oneshot receiver");

    // this will be an implementation detail of the application and
    // will define how often we probe tokio oneshot channels for the reply
    let mut interval = time::interval(Duration::from_millis(100));
    loop {
        tokio::select! {
            _ = interval.tick() => (),

            reply = &mut awaiting => {
                let reply = reply
                    .expect("failed to read oneshot channel")
                    .expect("failed to fetch response from handler");

                println!("response: {}", reply);

                break;
            }
        }
    }
}
```

It will produce the following output

```
Hello, "Victor"!
response: 16
```
