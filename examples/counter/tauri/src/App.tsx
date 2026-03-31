import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

type ViewModel = {
  count: string;
};

const initialState: ViewModel = {
  count: "",
};

function App() {
  const [view, setView] = useState(initialState);

  useEffect(() => {
    let unlistenToRender: UnlistenFn;

    listen<ViewModel>("render", (event) => {
      setView(event.payload);
    }).then((unlisten) => {
      unlistenToRender = unlisten;
    });

    // trigger initial render
    invoke("reset");

    return () => {
      unlistenToRender?.();
    };
  }, []);

  return (
    <main>
      <section className="section has-text-centered">
        <p className="title">Crux Counter Example</p>
        <p className="is-size-5">Rust Core, Rust Shell (Tauri + React)</p>
      </section>
      <section className="container has-text-centered">
        <p className="is-size-5">{view.count}</p>
        <div className="buttons section is-centered">
          <button
            className="button is-primary is-danger"
            onClick={() => invoke("reset")}
          >
            {"Reset"}
          </button>
          <button
            className="button is-primary is-success"
            onClick={() => invoke("increment")}
          >
            {"Increment"}
          </button>
          <button
            className="button is-primary is-warning"
            onClick={() => invoke("decrement")}
          >
            {"Decrement"}
          </button>
        </div>
      </section>
    </main>
  );
}

export default App;
