# dioxus-template

> a template for starting a dioxus project to be used with
> [dioxus-cli](https://dioxuslabs.com/learn/0.4/getting_started/wasm)

Note: you may need to update `dioxus-cli` first (to build against newer versions
of `wasm-bindgen`):

```
cargo install --force dioxus-cli
```

## Usage

#### use `dioxus-cli` init the template:

```
dx init hello-dioxus
```

or you can choose the template, for this template:

```
dx init hello-dioxus --template=gh:dioxuslabs/dioxus-template
```

#### Start a `dev-server` for the project:

```
cd ./hello-dioxus
dx serve
```

or package this project:

```
dx build --release
```

## Project Structure

```
.project
- public # save the assets you want include in your project.
- src # put your code
- - utils # save some public function
- - components # save some custom components
```
