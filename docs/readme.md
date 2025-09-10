
Easymake is a **declarative build system** based on YAML files called **`Emakefile`**.  
It allows you to define **targets** composed of **steps**, which execute reusable **actions** (such as running a shell command, downloading a file, or creating an archive).  

Unlike traditional build tools, Easymake emphasizes **simplicity**, **extensibility**, and **structured documentation**.  

---

## 🚀 Getting Started

### 1. Create your first project
Create a folder `my-project` and inside it create a file called `Emakefile`.

```yaml
targets:
  my_first_target:
    steps:
      - description: Say hello
        shell:
          cmd: echo "Hello !"
```

Run your target:

```sh
cd my-project
emake build //my_first_target
```

✅ You should see `Hello !` printed.  
Emake will also create a `.emake` folder in your project (learn more about it [here](./docs/emake_folder.md)).

---

### 2. Use variables
Variables make your Emakefile reusable and configurable.

```yaml
{% raw %}
variables:
  name: Linus

targets:
  my_first_target:
    steps:
      - description: Say hello
        shell:
          cmd: echo "Hello {{ variables:name }} !"
{% endraw %}
```

Run it and you’ll get:

```sh
Hello Linus !
```

You can also use variables inside `in_files` or other fields:

```yaml
{% raw %}
variables:
  name: Linus
  linux_readme_url: https://raw.githubusercontent.com/torvalds/linux/refs/heads/master/README

targets:
  my_first_target:
    steps:
      - description: Say hello
        shell:
          in_files:
            - "{{ variables:linux_readme_url }}"
          cmd: echo "Hello {{ variables:name }}! Linux README is here {{ in_files }}"
{% endraw %}
```

---

### 3. Add target dependencies
Targets can depend on each other. Let’s create an archive before saying hello:

```yaml
{% raw %}
variables:
  name: Linus
  linux_readme_url: https://raw.githubusercontent.com/torvalds/linux/refs/heads/master/README
  linux_credits_url: https://raw.githubusercontent.com/torvalds/linux/refs/heads/master/CREDITS

targets:
  prepare_archive:
    steps:
      - description: Prepare archive containing Readme and Credits
        archive:
          from:
            - "{{ variables:linux_readme_url }}"
            - "{{ variables:linux_credits_url }}"
          to: "{{ EMAKE_OUT_DIR }}/my_archive.tar.gz"

  my_first_target:
    deps:
      - prepare_archive
    steps:
      - description: Say hello
        shell:
          in_files:
            - "{{ EMAKE_OUT_DIR }}/my_archive.tar.gz"
          cmd: echo "Hello {{ variables:name }}! Archive is at {{ in_files }}"
{% endraw %}
```

---

### 4. Multiple steps in parallel
By default, steps run **sequentially**. You can set `parallel: true` when steps are independent:

```yaml
{% raw %}
targets:
  my_first_target:
    parallel: true
    steps:
      - description: Greet
        shell:
          cmd: echo "Hello {{ variables:name }}!"
      - description: Show archive
        shell:
          in_files:
            - "{{ EMAKE_OUT_DIR }}/my_archive.tar.gz"
          cmd: echo "Archive available at {{ in_files }}"
{% endraw %}
```

---

### 5. Split into multiple Emakefiles
Large projects can be split across multiple Emakefiles. Create the following structure for our example:

📁 Project structure:
```
my-project/
├─ Emakefile
└─ archive/
   └─ Emakefile
```

Inside `archive/Emakefile`:

```yaml
{% raw %}
targets:
  prepare_archive:
    steps:
      - description: Prepare archive containing Linux files
        archive:
          from:
            - "{{ //variables:linux_readme_url }}"
            - "{{ //variables:linux_credits_url }}"
          to: "{{ EMAKE_OUT_DIR }}/my_archive.tar.gz"
{% endraw %}
```
**Note** that we don't use variables from the root Emakefile but you are also able to create variables in this file.

Inside the root `Emakefile`:

```yaml
{% raw %}
variables:
  name: Linus
  linux_readme_url: https://raw.githubusercontent.com/torvalds/linux/refs/heads/master/README
  linux_credits_url: https://raw.githubusercontent.com/torvalds/linux/refs/heads/master/CREDITS

targets:
  my_first_target:
    deps:
      - //archive/targets:prepare_archive
    steps:
      - description: Say hello
        shell:
          cmd: echo "Hello {{ variables:name }}!"
{% endraw %}
```

---

## 🛠 Command Line

### Build a target
```sh
emake build [TARGET_PATH]
```
Use `--cwd [PATH]` to specify a project directory if not in the current folder.

### Clean
Removes the `.emake` folder.
```sh
emake clean
```

### Generate a dependency graph
```sh
emake graph [TARGET_PATH]
```

---

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
