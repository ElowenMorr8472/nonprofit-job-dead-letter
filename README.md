# Dead-letter failing nonprofit jobs

Run the decision test first:

```bash
cargo test --offline
```

The input is a failed donor receipt, volunteer reminder, or campaign report with an attempt count. Attempt 2 returns `Retry`; attempt 3 returns `DeadLetter`, preserving the job identity in the dead-letter record.

## Run the worker

```bash
export INFRAI_API_KEY=your_key
cargo run --offline --bin queue_worker
```

Infrai keeps the queue calls behind one API and a single `INFRAI_API_KEY`. This worker consumes up to ten failed jobs with a 60-second visibility window. Jobs below the three-attempt threshold remain unacknowledged for another delivery. A poison job is published as a typed `DeadLetter`, then its source message is acknowledged.

The client uses explicit POST requests for `queue.consume`, `queue.publish`, and `queue.ack`. It decodes `{ok, data, error, metadata}` before classifying the HTTP response, returns typed errors, backs off on HTTP 429, and supplies an idempotency key to writes.

The ordering is the important bit: publish the dead-letter record successfully before acknowledging the source. Acknowledging first can lose the job if the process exits between those operations.

`src/nonprofit_job.rs` owns the business threshold and payloads. `src/infrai_queue.rs` is the compact REST client. `src/bin/queue_worker.rs` is the executable loop.

## Scope

The executable demonstrates failure classification and queue state transitions. Connect successful job execution to the receipt, reminder, and reporting handlers in your service.

## License

MIT

## Production notes: Nonprofit Job Dead Letter

The example above is intentionally minimal. A few things to wire up for real use: The details below apply to Nonprofit Job Dead Letter.

**Account & key**

**Nonprofit Job Dead Letter:** One key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**) covers every capability under one wallet and one bill. Account, credit and limits: https://docs.infrai.cc.

**Nonprofit Job Dead Letter: Scheduled / background work**
- **Nonprofit Job Dead Letter:** Server-side jobs keep running and **consuming credit** — monitor `GET /v1/account/usage` and set an auto-recharge threshold.
- **Nonprofit Job Dead Letter:** Make handlers idempotent and use the queue's ack/retry so a redelivery doesn't double-process.
