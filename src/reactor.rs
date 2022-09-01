use std::collections::HashMap;
use std::sync::Arc;
use std::time;

use tokio::sync::{self, mpsc, oneshot};

pub use serde_json::Value;

use crate::{Id, Notification, Params, Request, Response, RpcError};

#[derive(Debug)]
struct PendingRequest {
    sender: oneshot::Sender<Result<Value, RpcError>>,
    moment: time::Instant,
    timeout: Option<time::Duration>,
}

#[derive(Debug)]
pub struct Reactor {
    capacity: usize,
    request_id: i64,
    requests: mpsc::Sender<Request>,
    notifications: mpsc::Sender<Notification>,
    pending: Arc<sync::RwLock<HashMap<Id, PendingRequest>>>,
}

impl Reactor {
    pub fn spawn(
        capacity: usize,
        requests: mpsc::Sender<Request>,
        notifications: mpsc::Sender<Notification>,
    ) -> (Self, mpsc::Sender<Response>) {
        let request_id = 0;
        let (responses_tx, mut responses) = mpsc::channel(capacity);

        let pending = HashMap::with_capacity(capacity);
        let pending = sync::RwLock::new(pending);
        let pending = Arc::new(pending);
        let pending_thr = Arc::clone(&pending);

        tokio::spawn(async move {
            while let Some(Response { id, result }) = responses.recv().await {
                let mut pending = pending_thr.write().await;

                if let Some(PendingRequest { sender, .. }) = pending.remove(&id) {
                    sender.send(result).ok();
                }
            }
        });

        let slf = Self {
            capacity,
            request_id,
            requests,
            notifications,
            pending,
        };

        (slf, responses_tx)
    }

    pub async fn notify<M, P>(
        &mut self,
        method: M,
        params: P,
        timeout: Option<time::Duration>,
    ) -> bool
    where
        M: AsRef<str>,
        P: Into<Params>,
    {
        let method = method.as_ref().to_string();
        let params = params.into();
        let notification = Notification { method, params };

        match timeout {
            Some(t) => self
                .notifications
                .send_timeout(notification, t)
                .await
                .is_ok(),

            None => self.notifications.send(notification).await.is_ok(),
        }
    }

    pub async fn request<M, P>(
        &mut self,
        method: M,
        params: P,
        timeout: Option<time::Duration>,
    ) -> Option<oneshot::Receiver<Result<Value, RpcError>>>
    where
        M: AsRef<str>,
        P: Into<Params>,
    {
        let id = self.request_id;

        self.request_id = id.wrapping_add(1);

        self.request_with_id(Id::Number(id), method, params, timeout)
            .await
    }

    pub async fn request_with_id<M, P>(
        &mut self,
        id: Id,
        method: M,
        params: P,
        timeout: Option<time::Duration>,
    ) -> Option<oneshot::Receiver<Result<Value, RpcError>>>
    where
        M: AsRef<str>,
        P: Into<Params>,
    {
        let method = method.as_ref().to_string();
        let params = params.into();
        let request = Request {
            id: id.clone(),
            method,
            params,
        };

        let sent = match &timeout {
            Some(t) => self.requests.send_timeout(request, *t).await.is_ok(),
            None => self.requests.send(request).await.is_ok(),
        };

        if !sent {
            return None;
        }

        let (sender, receiver) = oneshot::channel();
        let pending = PendingRequest {
            sender,
            moment: time::Instant::now(),
            timeout,
        };

        let mut queue = self.pending.write().await;

        queue.insert(id, pending);

        // attempt to clean expired pending responses
        if self.capacity < queue.len() {
            let now = time::Instant::now();

            let expired = queue
                .iter()
                .filter_map(|(id, pending)| {
                    pending.timeout.and_then(|t| {
                        let diff = now.duration_since(pending.moment);

                        (t < diff).then_some(id)
                    })
                })
                .cloned()
                .collect::<Vec<_>>();

            for id in expired {
                if let Some(pending) = queue.remove(&id) {
                    let response = Err(RpcError {
                        code: -1,
                        message: String::from("response timeout"),
                        data: Value::Null,
                    });

                    pending.sender.send(response).ok();
                }
            }
        }

        Some(receiver)
    }
}
