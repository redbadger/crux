/* @refresh reload */
import { render } from "solid-js/web";
import core, { update } from "./core";
import { EventVariantReset, EventVariantIncrement, EventVariantDecrement } from "shared_types/types/shared_types";
import "bulma/css/bulma.css";

function App() {
  return (
    <section class="box container has-text-centered m-5">
      <p class="is-size-5">{core.getView().count}</p>
      <div class="buttons section is-centered">
        <button class="button is-primary is-danger" onClick={() => update(new EventVariantReset())}>
          {"Reset"}
        </button>
        <button class="button is-primary is-success" onClick={() => update(new EventVariantIncrement())}>
          {"Increment"}
        </button>
        <button class="button is-primary is-warning" onClick={() => update(new EventVariantDecrement())}>
          {"Decrement"}
        </button>
      </div>
    </section>
  );
}

const root = document.getElementById("root");
render(() => <App />, root!);
