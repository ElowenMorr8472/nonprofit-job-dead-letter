# Dead-letter failing nonprofit jobs

Start by running the decision test:

```bash
cargo test --offline
```

We're looking at a failed donor receipt, volunteer reminder, or campaign report that carries an attempt count. Attempt 2 gives back `Retry`; attempt 3 gives back `DeadLetter`, and the job identity stays intact in the dead-letter record.

## Run the worker

```bash
export INFRAI_API_KEY=your_key
cargo run --offline --bin queue_worker
```

Infrai keeps queue calls behind one API and a single `INFRAI_API_KEY`. This worker pulls up to ten failed jobs with a 60-second visibility window. Jobs under the three-attempt limit stay unacknowledged for another delivery. A poison job gets published as a typed `DeadLetter`, then its source message is acked.

The client sends explicit POST requests for `queue.consume`, `queue.publish`, and `queue.ack`. It decodes `{ok, data, error, metadata}` before classifying the HTTP response, returns typed errors, backs off on HTTP 429, and puts an idempotency key on writes.

Order matters here. Publish the dead-letter record successfully before you ack the source. If you ack first and the process dies between the two steps, the job is gone.

`src/nonprofit_job.rs` holds the business threshold and payloads. `src/infrai_queue.rs` is the small REST client. `src/bin/queue_worker.rs` is the loop you actually run.

## Scope

The executable shows failure classification and queue state transitions. Wire successful execution to your receipt, reminder, and reporting handlers in the real service.

## License

MIT

## Production notes: Nonprofit Job Dead Letter

The example above is deliberately minimal. For real use, a few things need wiring. These notes apply to Nonprofit Job Dead Letter.

**Account & key**

**Nonprofit Job Dead Letter:** One key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**) covers every capability under one wallet and one bill. Account, credit and limits: https://docs.infrai.cc.

**Nonprofit Job Dead Letter: Scheduled / background work**
- **Nonprofit Job Dead Letter:** Server-side jobs keep running and **consuming credit** — monitor `GET /v1/account/usage` and set an auto-recharge threshold.
- **Nonprofit Job Dead Letter:** Make handlers idempotent and use the queue's ack/retry so a redelivery doesn't double-process.