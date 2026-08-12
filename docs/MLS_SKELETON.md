# MLS Skeleton

This document describes the current `MLS` layer in `lobster-chat`.

## Intent

The current implementation is a lifecycle skeleton, not a full cryptographic MLS implementation.

Its job is to stabilize:

- 1v1 bootstrap
- room bootstrap
- group membership changes
- epoch rotation
- encrypted payload boundary shape

So later we can swap in a real MLS implementation without redesigning the rest of the app.

## What exists now

The `crypto-mls` crate currently provides:

- `MlsGroupKind`
- `MlsMember`
- `MlsGroupState`
- `MlsCiphertextEnvelope`
- `SecureSessionManager`
- `InMemorySecureSessionManager`

## Current behavior

### Direct sessions

Direct sessions:

- require exactly 2 starting members
- default to `ConversationScope::Private`
- start at epoch `1`

### Room sessions

Room sessions:

- support non-private scopes
- can add and remove members
- set `pending_rekey = true` on membership changes
- rotate to a new epoch explicitly

### Ciphertext boundary

Right now the ciphertext boundary is:

- `MlsCiphertextEnvelope`
- encoded with a placeholder `SkeletonPostcard` wire format

This is intentionally just a transport boundary placeholder.
It is **not** the final cryptographic MLS wire format.

The gateway's lifecycle state is a separate at-rest boundary: `secure-sessions.json` is sealed with
AES-256-GCM under a key derived from `LOBSTER_SECURE_SESSION_MASTER_KEY`, and public session
projections omit `group_key`. This protects the current skeleton's persisted lifecycle material but
does not turn the skeleton into interoperable MLS or production end-to-end encryption.

## Why this is still useful

Even without final cryptography, this layer already gives the rest of the system stable expectations for:

- when sessions are created
- what a protected room or DM needs to know
- how epochs evolve
- how transport payloads should be wrapped

That lets the host adapters, H5 shell, and Waku transport grow against a stable security lifecycle.

## Next step

The next real upgrade for this layer should be:

- replace the postcard placeholder with real MLS-backed encrypted payloads
- keep the session manager and envelope boundary shape stable
