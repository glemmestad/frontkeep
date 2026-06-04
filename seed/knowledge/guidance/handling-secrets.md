# Handling secrets

## The short answer

**Secret values never live in code, config files, dotenv files, pull request descriptions, comments, commit messages, chat, logs, or any manifest.** They live in a secret store, and your code reads them at runtime through an identity that's been granted access to exactly those secrets and no others.

That's the whole rule. Everything below is the mechanics of following it and the reasoning for why there are no exceptions.

## Fetch at runtime, never embed

The pattern, every time, is the same: provision the secret through the catalog, get back a *pointer* to it, store the pointer wherever you like, and read the actual value at runtime when you need it.

```python
# The pointer (an identifier / reference) lives in config and is non-sensitive.
# The value is read at runtime through the granted identity.
secret_value = secret_store.get(reference=os.environ["SOMETHING_SECRET_REF"])
```

The value exists in memory only for as long as your process needs it. It's never written to disk, never logged, never serialized into anything that gets committed or shipped.

## Pointers are safe; values are not

The single mental model that makes this easy: there are two kinds of thing, and only one of them is sensitive.

- A **pointer** — a reference, an identifier, an ARN-style locator — tells you *where* a secret lives. It's not sensitive. An attacker holding the pointer still can't read the value without the identity and the grant. Pointers are safe to commit, log, paste in a PR, and put in environment variables. By convention, name them so it's obvious: anything ending in `_SECRET_REF` (or your platform's equivalent) is understood to be a pointer.
- A **value** — the actual key, password, or token — is the secret. It goes in exactly one place: the secret store. If you find yourself about to put a value anywhere else, you're operating at the wrong layer. Provision it properly, get the pointer, and read the value at runtime.

The corollary: nothing resembling `*_SECRET_VALUE`, `*_PASSWORD`, or a key field containing literal opaque bytes ever belongs in an environment variable, a manifest, or the repo. If it's in your hand as a literal, it's in the wrong place.

## Where secrets come from

You don't generate and paste secrets by hand. Catalog services that need a secret mint it in the secret store and hand back only the pointer. An identity provider integration returns a reference to its client credentials; a random-secret generator mints opaque material (signing keys, session secrets) and returns a reference; a managed database returns a reference to its auto-rotated credentials; the model gateway returns a reference to your project's API key. In every case the value is created where it should live, and you only ever touch the pointer.

If you genuinely need a secret no catalog service produces, the answer is to add a generator for it through the catalog — not to hand-roll a value and smuggle it into config. The pattern doesn't change; only which service mints the secret does.

## Least privilege

When your workload requests access, it declares the specific secrets it needs to read, and its runtime identity is granted read access to *exactly those* — no more. No wildcard grants, no "read all secrets in the project to keep it simple." The reason secrets never need to be logged or pasted is precisely this: the code reads them from the right place, through the right grant, every time. A wildcard grant throws that away and turns one compromised component into access to everything.

This is why "just put it in an env var to make it easy this once" is never the easy path. The proper path — provision, grant, read at runtime — is *already* the easy path once it's set up, and it's the one that doesn't leak.

## Rotation

Secrets get rotated — credentials expire, keys get cycled on a schedule, a leak forces an emergency rotation. Your code has to assume the value can change underneath it. So:

- **Don't cache a secret value for the lifetime of the process.** A long-running service that read its database password once at startup will break the moment that password rotates. Read on demand, or refresh periodically.
- **Know which of your secrets auto-rotate** and which you rotate manually, and make sure the manual ones actually get a calendar entry rather than living forever.

Designing for rotation from the start costs nothing. Retrofitting it onto a service that hardcoded a cached value at boot is a painful afternoon and usually a small outage.

## App-layer secrets vs. infra secrets

It's worth keeping two categories straight, because they're owned and rotated differently:

- **App-layer secrets** are the ones your application code consumes: third-party API keys, the model gateway key, an identity provider's client secret, an HMAC signing key. Your project owns these, provisions them through the catalog, and your code reads them at runtime. This guide is mostly about these.
- **Infra secrets** are credentials the platform uses to stand up and manage your infrastructure — deploy credentials, the secret store's own access path, database master credentials managed by the platform's rotation machinery. These are managed *below* your application; you reference the results (often as auto-rotated pointers) but you don't mint or hold them.

The rule is identical for both — value in the store, pointer in your hand, read at runtime — but knowing which layer a secret belongs to tells you who rotates it and where to look when something expires.

## The cultural point

The reason this is a hard line and not a guideline: the first value someone writes into the repo "just this once, to unblock myself" teaches the next person that it's fine, and the one after that. That's how a single shortcut becomes a culture, and how a culture becomes a leak. If you're tempted to commit a value to make something easier right now, stop — the cost of doing it right is one-time and small, and the cost of the leak lands on whoever's on call when it's discovered.

## See also

- `picking-a-classification` — higher data classes raise the bar on how secrets and access are handled.
- `choose-a-model-by-data-class` — anything touching a secret should be treated as at least confidential when choosing a model.
- `cost-optimization` — provisioning through the catalog (rather than personal accounts) is what keeps both secrets and spend governed.
