# World / City Interconnect API

This document describes the localhost world/city interconnect endpoints exposed by `lobster-waku-gateway`.

## Intent

The world/city layer treats the network as a world of cities and settlements:

- `World`: shared protocol, portability rules, and interconnect customs
- `City`: a self-governed settlement with relay, storage, and shared-commons stewardship responsibilities
- `Resident`: a portable identity that can move across cities
- `City Lord`: a city-scoped steward with strong shared-commons authority, but no access to private 1v1 plaintext and no global ownership over residents

There should also be future world-level feeds for:

- World Square public notices
- city trust / quarantine state
- deny lists and emergency advisories
- resident sanction / registration denylist feeds

The public city directory is expected to become mirrorable across multiple cities.
One city may act as an early seed city, but clients should be able to discover the world from any healthy mirror.

## Auth endpoints

### `POST /v1/auth/preflight`

Checks whether a proposed email/mobile pair is currently eligible for registration or login.

Request body:

```json
{
  "email": "reader@example.com",
  "mobile": "+86 13800138000"
}
```

Current behavior:

- normalizes email and mobile handles
- checks the world registration denylist
- returns `allowed=false` when a burned handle is reused

### `POST /v1/auth/email-otp/request`

Requests an email OTP challenge for a resident.

Request body:

```json
{
  "email": "reader@example.com",
  "mobile": "+86 13800138000",
  "resident_id": "reader-01"
}
```

Current behavior:

- blocks denied handles before issuing a challenge
- persists the OTP challenge in `auth-state.json`
- supports dev-mode inline OTP exposure through `LOBSTER_DEV_EMAIL_OTP_INLINE=1`

### `POST /v1/auth/email-otp/verify`

Verifies a pending email OTP and creates or refreshes a registration record.

Request body:

```json
{
  "challenge_id": "otp:gw-123",
  "code": "123456",
  "resident_id": "reader-01"
}
```

Current behavior:

- validates challenge freshness and code
- re-checks denylist state at verification time
- creates or refreshes a persisted resident registration
- is now consumed directly by the H5 resident auth lane so a verified `resident_id` can be rebound into the browser projection without a separate resident-account service

## Endpoints

### `GET /v1/world`

Returns the full world/city snapshot:

- `world`
- `portability`
- `cities`
- `memberships`
- `public_rooms`

This is the preferred bootstrap endpoint for world-aware residents and city-aware clients.

If the gateway is configured with an upstream gateway, the response can merge:

- local cities
- local memberships
- local public rooms
- upstream city catalog and public metadata

Longer term, this endpoint should also support:

- multiple upstream mirror sources
- signed snapshot metadata
- freshness timestamps
- mirror health hints
- signed world safety advisories
- city trust state and quarantine markers

### `GET /v1/shell/state`

Returns the gateway shell contract used by H5, TUI parity tests, and CLI-facing views.

Optional query:

```text
GET /v1/shell/state?resident_id=reader-01
```

Current behavior:

- every response includes `state_version`, a stable snapshot version derived from visible rooms, recent message metadata, recall metadata, and edit metadata
- clients can compare `state_version` between snapshots to skip unnecessary rerenders and detect read-side changes
- without `resident_id`, returns the unscoped shell projection used by non-resident debug/admin surfaces
- with `resident_id`, filters direct conversations to that resident and keeps public/visitor-visible rooms available
- direct conversation labels are generated from the viewer perspective, so `self_label`, `peer_label`, `participant_label`, and `title` describe the counterpart correctly for the current resident
- `creative.html` calls this endpoint with `resident_id=<current browser identity>` when connected to a real gateway; the default unauthenticated identity is `访客`
- `user.html` is only a query-preserving compatibility redirect to `creative.html` and must not carry an independent resident projection
- a verified email OTP login persists the returned `resident_id` in the browser and immediately reloads this endpoint for the resident-scoped conversation shell

### `GET /v1/shell/events`

Returns a Server-Sent Events stream for shell updates. The first data event is
always a `shell-state` snapshot with the same payload shape as
`GET /v1/shell/state`.

Optional query:

```text
GET /v1/shell/events?resident_id=reader-01
GET /v1/shell/events?resident_id=reader-01&after=<state_version>&wait_ms=1000
```

Current behavior:

- the stream advertises `retry: 4000`
- event type: `shell-state`
- data payload: JSON `ShellState`, including `state_version`
- heartbeat event type: `shell-heartbeat`
- heartbeat data payload: `{ "now_ms": <unix-ms>, "resident_id": <id|null> }`
- `resident_id` scoping matches `GET /v1/shell/state`
- if `after` and `wait_ms` are provided, the gateway waits until `state_version` changes or the timeout expires before returning the next `shell-state`
- if the timeout expires without a state change, the gateway returns the current snapshot with the same `state_version` plus a `shell-heartbeat`; this is not a send failure
- successful shell writes, edits, recalls, and CLI sends notify waiting event requests; clients should still treat each response as one snapshot and reconnect with the latest `state_version`
- `wait_ms` is clamped to a maximum of 5000 ms
- response content type is `text/event-stream; charset=utf-8`
- response disables caching with `Cache-Control: no-cache`

This is the gateway contract for the H5 realtime path. Browser clients should
prefer this stream for read-side refreshes and keep `POST /v1/shell/message` for
write-side sends.

### `POST /v1/shell/message`

Appends a browser shell message into the current conversation.

Current behavior:

- with `LOBSTER_DEV_AUTH_BYPASS` disabled, the request must include a valid `Authorization: Bearer <session_token>` whose resident identity matches `sender`
- `LOBSTER_DEV_AUTH_BYPASS=1` is reserved for local synthetic-identity fixtures; it is not a production authentication mode
- `sender` must be a registered resident or agent identity
- the unauthenticated browser identity `访客` is rejected with a login-required error instead of being accepted into the timeline
- city room posts still pass through the normal room membership/freeze checks after the sender is authenticated
- `text` is trimmed and must contain 1-2000 characters after trimming
- requests may include `reply_to_message_id`; when present, the successful response and later shell message projection include the same reference
- successful responses expose real delivery metadata: `ok`, `delivery_status`, `conversation_id`, `message_id`, `delivered_at_ms`, `sender`, and canonical trimmed `text`
- later `GET /v1/shell/state` and `GET /v1/shell/events` message projections include `delivery_status`; committed gateway messages currently project as `delivered`
- `delivery_status="delivered"` means the gateway accepted and published the message; clients should treat any non-2xx response as the real failure state instead of inventing a local success

### `POST /v1/shell/message/recall`

Marks a browser shell message as recalled without deleting the timeline audit entry.

Request shape:

```json
{
  "room_id": "room:world:lobby",
  "message_id": "gw-1777893587078-4",
  "actor": "qa-a"
}
```

Current behavior:

- `actor` must be an authenticated resident or agent identity
- only the original sender can recall the message
- missing messages and unauthorized recall attempts return non-2xx errors
- successful responses expose `ok`, `conversation_id`, `message_id`, `recall_status`, `recalled_at_ms`, and `recalled_by`
- recalled messages remain in `GET /v1/shell/state` / `GET /v1/shell/events` projections with `is_recalled=true`, `recalled_by`, `recalled_at_ms`, and display text `消息已撤回`
- recall metadata changes `state_version`, so SSE clients using `after=<state_version>` can observe the update

### `POST /v1/shell/message/edit`

Edits a browser shell message in place while keeping the original `message_id`.

Request shape:

```json
{
  "room_id": "room:world:lobby",
  "message_id": "gw-1777893587078-4",
  "actor": "qa-a",
  "text": "updated message text"
}
```

Current behavior:

- `actor` must be an authenticated resident or agent identity
- only the original sender can edit the message
- `text` is trimmed and must contain 1-2000 characters after trimming
- recalled messages cannot be edited
- successful responses expose `ok`, `conversation_id`, `message_id`, `edit_status`, `edited_at_ms`, `edited_by`, and canonical trimmed `text`
- edited messages remain in `GET /v1/shell/state` / `GET /v1/shell/events` projections with the same `message_id`, updated `text`, `is_edited=true`, `edited_by`, and `edited_at_ms`
- edit metadata changes `state_version`, so SSE clients using `after=<state_version>` can observe the update

### CLI read-side metadata

`GET /v1/cli/tail` uses the same timeline truth as the H5 shell projection. Each tail message includes:

- `message_id`, `sender`, `text`, `timestamp_ms`
- `is_recalled`, plus optional `recalled_by` and `recalled_at_ms`
- `is_edited`, plus optional `edited_by` and `edited_at_ms`

Current behavior:

- recalled CLI tail messages display canonical text `消息已撤回`
- edited CLI tail messages keep the same `message_id` and expose the updated text plus edit metadata
- `GET /v1/cli/inbox` uses the canonical recalled text for `last_message_preview` when the newest message has been recalled
- `POST /v1/cli/send` shares the shell text contract: trim first, reject empty text, reject text over 2000 characters
- with `LOBSTER_DEV_AUTH_BYPASS` disabled, `from: user:<id>` requires a matching resident Bearer session, while `from: agent:<id>` requires a Bearer token bound by `LOBSTER_AGENT_TOKENS=agent:<id>=<token>[,agent:<id>=<token>...]`
- synthetic `qa-*` CLI smoke identities explicitly start the gateway with `LOBSTER_DEV_AUTH_BYPASS=1`; this is not a production authentication mode
- missing or mismatched CLI sender authentication returns `401`
- `lobster-cli edit` posts to `POST /v1/shell/message/edit`
- `lobster-cli recall` posts to `POST /v1/shell/message/recall`
- CLI edit/recall requests preserve the typed `actor_address` (`user:<id>` or `agent:<id>`); use `--token` (or cached `LOBSTER_SESSION_TOKEN`) for users and `--agent-token` (or `LOBSTER_AGENT_TOKEN`) for agents. Legacy browser shell requests may omit `actor_address` and continue using their existing session authentication.
- CLI/TUI clients should not reconstruct recall or edit status locally; they should render these fields from the gateway response

### `GET /v1/cities`

Returns only city definitions. Useful for lightweight clients that want discovery without membership detail.

This endpoint is also the natural base for mirrored world-directory snapshots.

Longer term, clients should be able to distinguish:

- healthy cities
- cities under review
- quarantined cities
- isolated cities

### `GET /v1/residents`

Returns a lightweight resident directory derived from memberships.

Each entry currently contains:

- `resident_id`
- `active_cities`
- `pending_cities`
- `roles`

This is intended as the bridge for:

- H5 direct-message doorways
- resident lists inside a city
- later themed honorific rendering

### `GET /v1/world-square`

Returns the mirrored public notice feed for the World Square commons. In the current H5 product line, `world-square.html` consumes this as a readonly public-square projection rather than a full forum model.

This is intended for:

- world-level announcements
- cross-city public notices
- migration and incident notices
- lightweight readonly H5 summaries for `公告` / `公共讨论` / `跨城发现`

Each notice currently carries:

- `notice_id`
- `title`
- `body`
- `author_id`
- `posted_at_ms`
- `severity`
- `tags`

The endpoint must not be used as an IM source of truth. `conversation_shell` / `scene_render` remain the chat contracts; world-square data is only a public information projection. H5 must render these fields as plain text and keep all world-square cards readonly.

### `GET /v1/world-safety`

Returns the world safety snapshot:

- world trusted-resident identities
- city trust state
- safety advisories
- safety reports
- known directory mirrors

This is the city-facing and resident-facing feed for:

- quarantine state
- emergency deny signals
- world-safety visibility

### `GET /v1/world-safety/reports`

Returns the public incident intake / review feed for world trusted residents.

Each report records:

- who reported the issue
- which public target was reported
- optional city binding
- evidence pointers
- review status

This feed exists so the world can react to abusive cities without reading private 1v1 plaintext.

### `GET /v1/world-snapshot`

Returns a single bundle for mirror nodes and lightweight projections:

- `meta`
  - `snapshot_id`
  - `generated_at_ms`
  - `world_id`
  - `world_title`
  - `checksum_sha256`
- `payload.governance`
- `payload.residents`
- `payload.directory`
- `payload.square`
- `payload.safety`
- `payload.mirror_sources`

This is the preferred endpoint for:

- mirror-city bootstrap
- client-side cached world snapshots
- quick integrity checks before applying a refreshed directory/safety feed
- H5 cold-start loading, so the page can fetch world, directory, residents, square, safety, and mirror-source state in one round trip

### `GET /v1/world-entry`

Returns a compact H5-facing projection for the `unified.html` world-entry metro station.

The response is derived from the federated world directory and contains:

- `title` and `station_label` for the world-entry HUD
- `current_city_slug`, currently `core-harbor`
- `source_summary` with route, mirror, notice, and advisory counts
- `routes[]` route cards with `title`, `description`, `href`, `status_label`, trust state, mirror state, counts, source kind, and `is_current`

This endpoint is intended for replacing the local placeholder route list in `unified.html` without pulling the full governance bundle into the visual page.

### `POST /v1/cities`

Creates a new city.

Request body:

```json
{
  "title": "Aurora Hub",
  "slug": "aurora-hub",
  "description": "Experimental city for relay and AI-sidecar services.",
  "lord_id": "rsaga"
}
```

Notes:

- `slug` is optional; it will be normalized from `title` if omitted.
- the creator becomes `CityRole::Lord`
- cities are persisted to the gateway state directory

### `POST /v1/cities/join`

Creates a membership for a resident.

Request body:

```json
{
  "city": "core-harbor",
  "resident_id": "guest-01"
}
```

`city` accepts either:

- full city id, such as `city:core-harbor`
- city slug, such as `core-harbor`

Join state depends on city policy:

- `Active` if approval is not required
- `PendingApproval` if the city requires approval

### `POST /v1/direct/open`

Bootstraps or reuses a direct 1v1 session and creates a private conversation entry if needed.

With development auth bypass disabled, `requester_id` must match the resident identity in a valid
`Authorization: Bearer <session_token>` header. The peer identity is not used as the caller identity.

Request body:

```json
{
  "requester_id": "rsaga",
  "requester_device_id": "desktop-1",
  "peer_id": "builder",
  "peer_device_id": "browser"
}
```

Rules:

- the conversation id is normalized to avoid duplicate `dm:a:b` vs `dm:b:a` rooms
- an existing direct session is reused if one already exists
- MLS skeleton session state is persisted
- the resulting conversation appears in shell state even before the first message is sent

### `POST /v1/cities/rooms`

Creates a public city room.

Request body:

```json
{
  "city": "core-harbor",
  "creator_id": "rsaga",
  "title": "Announcements",
  "slug": "announcements",
  "description": "Public city notices and service status."
}
```

Rules:

- the creator must have `CreatePublicRoom`
- current permission model grants this to `CityRole::Lord`
- rooms become regular `Conversation` entries and appear in shell state

### `POST /v1/cities/approve`

Approves a pending resident join request.

Request body:

```json
{
  "city": "approval-bay",
  "actor_id": "rsaga",
  "resident_id": "guest-01"
}
```

Rules:

- the actor must have `ApproveResidentJoin`
- the current model gives this to `CityRole::Lord`
- successful approval moves membership state to `Active`

### `POST /v1/cities/stewards`

Grants or revokes `Steward` role inside a city.

Request body:

```json
{
  "city": "core-harbor",
  "actor_id": "rsaga",
  "resident_id": "helper",
  "grant": true
}
```

Rules:

- `grant = true` requires `AssignSteward`
- `grant = false` requires `RevokeSteward`
- current model gives both to `CityRole::Lord`

### `POST /v1/cities/rooms/freeze`

Freezes or unfreezes a public room.

Request body:

```json
{
  "city": "core-harbor",
  "actor_id": "rsaga",
  "room": "lobby",
  "frozen": true
}
```

Rules:

- actor must have `FreezeRoom`
- current model gives this to `CityRole::Lord`
- when a room is frozen, ordinary residents can no longer post into that room
- city trusted residents with public-space authority can still post service or announcement messages

### `POST /v1/world-square/notices`

Publishes a signed-by-actor world-square notice.

Request body:

```json
{
  "actor_id": "rsaga",
  "title": "Mirror sync window",
  "body": "Directory mirrors will resync at dusk.",
  "severity": "warning",
  "tags": ["world", "mirror"]
}
```

Rules:

- actor must be a world steward
- this is for public world-layer notices, not private room-side handling

### `POST /v1/world-safety/cities/trust`

Updates the trust state of a city.

Request body:

```json
{
  "actor_id": "rsaga",
  "city": "core-harbor",
  "state": "UnderReview",
  "reason": "interconnect anomaly"
}
```

Rules:

- actor must be a world steward
- used for `Healthy`, `UnderReview`, `Quarantined`, `Isolated`
- non-healthy transitions can emit safety advisories

### `POST /v1/world-safety/reports`

Submits a world-safety report against a public target.

Request body:

```json
{
  "reporter_id": "guest-01",
  "city": "bad-harbor",
  "target_kind": "room",
  "target_ref": "room:city:bad-harbor:lobby",
  "summary": "public room is broadcasting illegal scam links",
  "evidence": ["https://example.invalid/evidence"]
}
```

Rules:

- any resident can submit a report
- reports are about public targets and public evidence
- this is the first step of the world safety review loop

### `GET /v1/world-safety/residents`

Returns the current resident-sanction feed and hashed registration denylist feed.

Response shape:

```json
{
  "resident_sanctions": [
    {
      "sanction_id": "resident-sanction:123",
      "resident_id": "bad-actor",
      "city_id": "city:bad-harbor",
      "report_id": "report:123",
      "reason": "confirmed organized scam operation",
      "portability_revoked": true,
      "status": "Active",
      "issued_by": "rsaga",
      "issued_at_ms": 1773700000000,
      "lifted_at_ms": null
    }
  ],
  "registration_blacklist": [
    {
      "entry_id": "blacklist:123",
      "resident_id": "bad-actor",
      "report_id": "report:123",
      "handle_kind": "email",
      "hash_sha256": "…",
      "reason": "confirmed organized scam operation",
      "added_by": "rsaga",
      "added_at_ms": 1773700000000
    }
  ]
}
```

Rules:

- this feed is world-scoped, public to compliant mirrors, and intended for inter-city trust enforcement
- email/mobile handles are published only as hashes, never as plaintext
- a resident sanction can revoke cross-city portability without exposing private message content

### `POST /v1/world-safety/reports/review`

Reviews a safety report and can optionally push the linked city into a new trust state.

Request body:

```json
{
  "actor_id": "rsaga",
  "report_id": "report:123",
  "status": "Resolved",
  "resolution": "confirmed abuse; quarantine city",
  "city_state": "Quarantined"
}
```

Rules:

- actor must be a world steward
- reviewing a report can trigger city quarantine/isolation
- this is the operational bridge from incident intake to interconnect cut-off
- the H5 shell can now drive this endpoint through the steward review form

### `POST /v1/world-safety/advisories`

Publishes a world-safety advisory.

Request body:

```json
{
  "actor_id": "rsaga",
  "subject_kind": "provider",
  "subject_ref": "http://bad-node.example",
  "action": "deny-link",
  "reason": "malware distribution",
  "expires_at_ms": null
}
```

Rules:

- actor must be a world steward
- intended for emergency public safety signals and deny-link publication

### `POST /v1/world-safety/residents/sanction`

Publishes a resident-sanction record and optionally adds hashed registration handles to the denylist feed.

Request body:

```json
{
  "actor_id": "rsaga",
  "resident_id": "bad-actor",
  "city": "bad-harbor",
  "report_id": "report:123",
  "reason": "confirmed organized scam operation",
  "email": "bad.actor@example.com",
  "mobile": "+86 13800138000",
  "portability_revoked": true
}
```

Rules:

- actor must be a world steward
- severe sanctions should normally be tied to a reviewed public safety report
- `email` and `mobile` are converted into hashed denylist entries before persistence
- once `portability_revoked` is active, the resident cannot join another city with the same world identity

## Persistence

The gateway stores world/city state in:

- `<state-dir>/governance-state.json`

The gateway stores the sealed MLS skeleton session state in:

- `<state-dir>/secure-sessions.json`

The file is encrypted with the key derived from `LOBSTER_SECURE_SESSION_MASTER_KEY` and must not be
copied without the matching deployment secret. The `POST /v1/direct/open` response contains only
group metadata; it never exposes `group_key`. Legacy plaintext snapshots are accepted once for
startup migration and are atomically replaced with a sealed snapshot.

and timeline state in the file-backed `FileTimelineStore` under the same root.

## Security boundary

The current policy is intentionally strict:

- city public-space authority is strong for public city spaces
- city-lord authority is at least comparable to a WeChat group owner for city rooms
- private 1v1 plaintext is not part of city public-space authority
- resident identity remains portable across cities

The current world-layer safety workflow can already:

- publish signed-style advisories
- delist or quarantine a malicious city from public world mirrors
- publish resident-sanction records and hashed registration denylist entries
- block cross-city joins for world-banned identities

Longer term, it should also be able to:

- delist or quarantine a malicious city from public world mirrors
- deny-link dangerous providers and public room pointers
- block a city from World Square participation
- lift or expire sanctions with review records and mirror signatures

while still not gaining access to private-message plaintext.

## Registration direction

Early-stage registration should keep cost low:

- collect mobile number
- collect email address
- send verification code by email instead of SMS

This preserves a real registration layer while avoiding SMS cost in the early phase.

For severe confirmed abuse, the world safety workflow may publish:

- resident identity revocations
- hashed email/mobile denylist entries

so that compliant cities can refuse cross-city portability for banned identities.
