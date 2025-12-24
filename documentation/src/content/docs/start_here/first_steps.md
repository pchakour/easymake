---
title: First step
description: Create your first project
---



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
