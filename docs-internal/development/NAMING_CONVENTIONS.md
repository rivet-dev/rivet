# Naming Conventions

> **Most internal APIs are not standardized**
>
> For reference, follow the conventions in `api-actor`.

## APIs, Operations, & Workflows

### Internal & external names

Internal & external names are often different. Convert the names on the API server. Do not expose internal names to the API. Make sure formatted errors also comply to the same type names.

See alias mappings [here](./INTERNAL_EXTERNAL_ALIASES.md).

### Legacy terms

- `{self}_id` -> `id` (e.g. `Actor.actor_id` -> `Actor.id`)
- `{some_type}_id` -> `{some_type}` (e.g. `Actor.datacenter_id` -> `Actor.datacenter`)
- `name_id` -> `slug`
- `display_name` -> `name`
- `{}_ts` -> `{}_at` (e.g. `created_ts` -> `created_at`)

### Request & response parameters

- Timestamps end with `at`. Use the past tense of the verb. For example, `createdAt` and `expiredAt`.
- Prefer enums instead of boolean. For example, instead of `completed: boolean`, use `status: "completed" | "started"`.
- Use optional timestamps instead of booleans. For example, instead of `completed: boolean`, use `completedAt?: number`.
- Refer to the ID of the current object as just `id` (not something like `user_id`).
- Do not use `id` in the variable name if passing an id. For example, use `datacenter` instead of `datacenter_id`.
    - If storing an ID, make sure that the variable name references the name of the corresponding data. For example `owner_user` -> `User` or `datacenter` -> `Datacenter`.
- Format dates with RFC 3339 (which is a profile of ISO 8601) in the format of `2024-07-06T04:56:49.517Z`. This provides a balance of human-readable & machine-readable dates.
    - Do not include timezones in dates. Always use UTC with `Z` (not `+00:00`).
    - Include milliseconds in dates.
- Common property names:
    - `id` Sometimes a UUID, sometimes a slug. e.g. datacenters use a string for the ID, but actors use a UUID. This is because actors are created by users & aren't usually referenced by name.
    - `created_at` & `destroyed_at`
    - `name` Used for human-readable name.
    - `slug` If there is a short string used to refer to this object. Only use this if not already used for the UUID. e.g. datacenters use a string for the ID.
- Format durations in milliseconds, except in the uncommon case of performance-sensitive code. In that case, use microseconds.
- For any other cases, delegate to [Stripe's naming semantics](https://docs.stripe.com/api/customers).

### Camel case + acryonyms

Follow camel case strictly & treat acronyms as single words.

Examples:

- Prefer `Uuid` instead of `UUID`
- Prefer `OpenApi` instead of `OpenAPI`

### Externally tagged enums

When representing an enum with associated data (often called "sum types" which are a kind of algebreic data type, ADT), represent using a nested object (often called "externally tagged enums").

This comes at the expense of not having exhaustive switch statements.

Externally tagged enums are easy to represent in languages that don't support advanced type constraints, such as C# and most OpenAPI SDK generators (i.e. don't support `oneOf`).

This example:

```typescript
type MyEnum = { foo: MyEnumFoo } | { bar: MyEnumBar } | { baz: MyEnumBaz };

interface MyEnumFoo {

}

interface MyEnumBar {

}

interface MyEnumBaz {

}
```

Can be represented in C# like this:

```csharp
class MyEnum {
    MyEnumFoo? Foo;
    MyEnumBar? Bar;
    MyEnumBaz? Baz;
}

class MyEnumFoo {
}

class MyEnumBar {
}

class MyEnumBaz {
}
```

## Databases

### Uses of `id` included with type

When referring to the ID of the current type, use `id`. When referring to a
foreign type, use `{type name}Id`.

