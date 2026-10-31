# Chat State Machine — GUI-P02 AI Chat Interface

## 1. Lifecycle States of a Chat Request

```
             [ User Submits Prompt ]
                        │
                        ▼
                   ┌──────────┐
                   │  Queued  │
                   └────┬─────┘
                        │ (ChatEvent::Started)
                        ▼
                 ┌──────────────┐
       ┌─────────┤  Streaming   ├────────┐
       │         └──────┬───────┘        │
       │                │                │
(ChatEvent::    (ChatEvent::      (ChatEvent::
 Cancelled)      Completed)         Failed)
       ▼                ▼                ▼
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│  Cancelled   │ │  Completed   │ │    Failed    │
└──────────────┘ └──────────────┘ └──────┬───────┘
                                         │ (User clicks Retry)
                                         ▼
                                   [ Re-submits ]
```

## 2. Invariant Rules
1. A message never enters `Completed` unless an explicit `ChatEvent::Completed` arrives.
2. In `Cancelled` state, previously accumulated deltas are strictly preserved.
3. In `Failed` state, the error detail is captured and a `Retry` action is enabled.
4. If the active request is cancelled, late arriving `Delta` events for that request are dropped.
