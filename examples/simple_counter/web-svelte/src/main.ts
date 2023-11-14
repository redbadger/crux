import "reflect-metadata";

import App from "./App.svelte";

document.body.setAttribute("data-app-container", "");

export default new App({ target: document.body });
