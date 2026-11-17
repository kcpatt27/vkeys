# T5 — rotate all, and an honest inventory

**Status:** planning. Follows T4 (transport) and the lost-device case.

## The real friction is not the command

Rotating keys is not the hard part. The hard part is the number of provider
dashboards you have to click through, and the number of accounts an agent has to
have access to in order to do it. Most credentials already live in a password
manager; what is missing is the **API key manager** and a way for agents to use
what it holds.

So the rotation feature is a cockpit, not a loop:

- Every name carries a **rotation URL** (the provider page that issues or
  revokes that key). `vkeys rotate` prints the link before prompting.
- `--all` groups by provider so one dashboard visit covers several keys.
- The password manager is out of scope. Do not import password-manager exports;
  do not try to be one.
- What matters is: agent can use the key, human never sees it, and the provider
  page is one click away when it must be reissued.

## Rotate all

A lost laptop means every key that device could read is suspect. Re-entering
them one command at a time is how rotation does not happen.

```
vkeys rotate --all [--since <epoch>] [--resume] [--abort-on-error]
```

- Iterates every name, prompting with a hidden paste per key. Never prints a
  value.
- `--since` limits to keys last rotated before a rotation epoch (the lost-device
  cut-off). `--resume` continues after an interrupted run.
- Skipping a key is allowed and recorded as `skipped`, not as done.
- On completion, increments a vault **rotation epoch**. Bundles and device
  registrations record the epoch they were created under.
- A key cannot be "rotated" without a new value; the command's job is to make
  the re-entry fast, not to invent credentials.

## Inventory states

`vkeys ls` should answer "what needs attention" without a second command:

| State | Meaning |
|---|---|
| `current` | rotated at or after the active epoch |
| `stale` | last rotated before the active epoch (suspect after a lost device) |
| `skipped` | explicitly deferred during a rotation run |
| `revoked` | name retired; keep for audit, never inject |

Never show an "empty" value state: a name with no value should not exist. If the
backend returns one, that is a vault error, not a row.

## Colour, stably

- Colour is applied **only when stdout is a TTY**, and only the state token, not
  the name.
- `--no-color` and `NO_COLOR` always win. `TERM=dumb` disables.
- Piped output is plain text, so scripts and agents never parse ANSI.
- Colour is a hint, never the only signal: the state word is always present.

## Acceptance

1. `rotate --all` prompts for every stale key, resumes after Ctrl-C, and records
   skips without marking them done.
2. `ls` shows the four states; a piped `ls` contains no ANSI bytes.
3. `NO_COLOR=1` and `--no-color` produce byte-identical output.
4. After a device revocation and epoch bump, every previously shared key reads
   `stale` until rotated.
5. No value, fingerprint, or length is printed at any point.
6. Every name can carry a rotation URL; `rotate` prints it before the prompt.
