---
title: Create targets
description: Create targets to build your project
sidebar:
    order: 2
---
Targets are declared in the top-level `targets` section of an `Emakefile`.

Each target is a mapping keyed by its name and may include several properties:

- `deps` (optional): a list of paths to other targets this target depends on.
- `parallel_deps` (optional, default: `true`): whether dependencies should be executed concurrently.
- `steps` (required): an ordered list of actions that build the target.
- `parallel_steps` (optional, default: `false`): whether steps within the target should run concurrently.

## Steps and Actions

A step typically contains a `description` and exactly one action to perform. Actions are defined by name and accept action-specific properties (for the full list see the actions reference: [/guides/actions/](../../guides/actions/)).


### Simple example with shell action

```yaml
targets:
  build:
    deps: [clean]
    steps:
      - description: Compile sources
        shell:
          cmd: cargo build --release

  clean:
    steps:
      - description: Remove build artifacts
        shell:
          cmd: cargo clean
```

### Parallel dependencies and steps

Use `parallel_deps: false` to preserve a strict dependency ordering when order matters. Use `parallel_steps: true` to run independent steps concurrently inside a target.

```yaml
targets:
  test_all:
    deps: [/test/unit, /test/integration]
    parallel_deps: false

  build_matrix:
    steps:
      - description: Build variant A
        shell: { cmd: cargo build --features=variant_a }
      - description: Build variant B
        shell: { cmd: cargo build --features=variant_b }
    parallel_steps: true
```

## Best practices

- Keep targets small and focused — compose complex workflows using deps.
- Prefer descriptive `description` fields for each step to make logs readable.
- Avoid long-running unrelated steps in a single target; split them into separate targets and use deps.
- Use `variables` for repeated values (paths, versions) and `secrets` for sensitive data.

## Troubleshooting

- Missing or mistyped action name: ensure the step defines a supported action and correct properties.
- Dependency cycle: circular `deps` will cause the graph execution to fail — break cycles by refactoring targets.
- Unexpected parallelism: if ordering matters, set `parallel_deps: false` or `parallel_steps: false`.

## See also

- Actions reference: [/guides/actions/](../../guides/actions/)
- Paths and scoping: [/guides/paths/](../../guides/paths/)