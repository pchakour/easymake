# Easymake

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
Emake will also create a `.emake` folder in your project (learn more about it [here](https://pchakour.github.io/easymake/emake_folder.html)).

---

## Documentation

Full documentation is available [here](https://pchakour.github.io/easymake/).

You’ll find detailed instructions on configuration and examples in the documentation.

---

## Contributing

Contributions are welcome! Please follow these steps:

1. Fork the repository.
2. Create a branch (`git checkout -b feature/my-feature`).
3. Make your changes.
4. Commit and push.
5. Open a Pull Request.

For major changes, please open an issue first to discuss what you’d like to change.

---

## License

This project is licensed under the **GPL-3.0 License**. See [LICENSE](LICENSE) for details.