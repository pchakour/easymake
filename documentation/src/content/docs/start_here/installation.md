---
title: Installation
description: Installation guide
---


## 🚀 Getting Started


## 📂 Project Structure

- Root `Emakefile`
- Optional subdirectories each with their own `Emakefile`
- Targets referenced using `//path/to/file/targets:target` or `//path/to/file/target`

---
## Paths

Paths in Emake are used to reference **variables**, **secrets**, and **targets** across your project structure.  
They follow a simple convention inspired by filesystem paths.

---

### Absolute vs Relative paths

- **Absolute paths**  
  Start with **`//`** and always point from the **root `Emakefile`**.  
  Example:  
  ```yaml
  //archive/other-folder/variables:firstname
  ```

- **Relative paths**  
  Start with a single **`/`** and are resolved **relative to the current `Emakefile`**.  
  Example:  
  ```yaml
  /archive/other-folder/variables:firstname
  ```

- **Local paths**  
  If you omit the leading slash, the reference stays **within the current `Emakefile`**.  
  Example:  
  ```yaml
  variables:firstname
  ```

---

### Example project structure

```
my-project/
├─ Emakefile
└─ archive/
   ├─ Emakefile
   └─ other-folder/
      └─ Emakefile
```

- From the **root `Emakefile`** to the `other-folder` one:
  - Relative path → `/archive/other-folder/`
  - Absolute path → `//archive/other-folder/`

---

### Referencing elements

At the end of a path, you must specify the **section** and the **element name**:  

- **Variable**  
  ```yaml
  //archive/other-folder/variables:firstname
  ```

- **Secret**  
  ```yaml
  //archive/other-folder/secrets:my-deep-secret
  ```

- **Target**  
  ```yaml
  //archive/other-folder/targets:my-best-target
  ```

> **Shortcut for targets:**  
> You can omit the `targets` section since it is the default.  
> Example:  
> ```yaml
> //archive/other-folder/my-best-target
> ```

---

### Summary

- `//` → start from root  
- `/` → go down into subfolders from current file  
- no prefix → stay in the same `Emakefile`  
- `section:name` → point to a **variable**, **secret**, or **target**  
- omit `targets:` → directly reference a target

---

## 📜 Emakefile Syntax

An `Emakefile` consists of three main sections:

### Variables

Reusable values for commands, URLs, or paths.
```yaml
variables:
  version: 1.0
```

Call them with the correct path:
```yaml
{% raw %}
cmd: echo "Version {{ variables:version }}"
{% endraw %}
```

### Secrets

Secrets are supported for managing credentials (see [secrets documentation](./secrets.md)).

### Targets

The core of your build.  
Each target has:
- `deps`: dependencies (other targets)
- `steps`: actions to execute
- `parallel`: (optional) run steps concurrently

Example:
```yaml
targets:
  build:
    deps: [clean]
    steps:
      - description: Compile
        shell:
          cmd: cargo build
```

---

## 📖 Next Steps
- Learn about all available actions: [Actions Reference](./actions.md)  
- Understand the `.emake` folder: [Emake Folder](./emake_folder.md)
