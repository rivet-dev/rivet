# Creating a formatted error

## Basic

```md
---
name = "MY_ERROR_NAME"
description = "My error description"
http_status = 400
---

# My Error

Write anything you want
```

## With procedural formatting

```md
---
name = "MY_ERROR_NAME"
description = "My error description with {arg1} and {arg2}"
http_status = 500
---

# My Error

Write anything you want
```

When `description_basic` is set and the error isn't built with `bad_request_builder` but instead just `bad_request`, `description_basic` will be used as the error description instead of `description`.

```md
---
name = "MY_ERROR_NAME"
description = "My error description with {arg1} and {arg2}"
description_basic = "My error description"
http_status = 500
---

# My Error

Write anything you want
```
