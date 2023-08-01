# dioxus-template

> a template for starting a dioxus project to be used with [dioxus-cli](https://github.com/DioxusLabs/cli)

## Usage

#### use `dioxus-cli` init the template:

```
dioxus init hello-dioxus
```

or you can choose the template, for this tempalte:

```
dioxus init hello-dioxus --template=gh:dioxuslabs/dioxus-template
```

#### Start a `dev-server` for the project:

```
cd ./hello-dioxus
dioxus serve
```

or package this project:

```
dioxus build --release
```

## Project Structure

```
.project
- public # save the assets you want include in your project.
- src # put your code
- - utils # save some public function
- - components # save some custom components
```
