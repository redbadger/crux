import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

type State = {
  text: string;
};

const initialState: State = {
  text: "",
};

function App() {
  const [view, setView] = useState(initialState);

  useEffect(
    () => {
      invoke("watch");
    },
    /*once*/ [],
  );

  useEffect(() => {
    let unlistenToRender: UnlistenFn;

    listen<State>("render", (event) => {
      setView(event.payload);
    }).then((unlisten) => {
      unlistenToRender = unlisten;
    });

    return () => {
      unlistenToRender?.();
    };
  });

  return (
    <main>
      <section className="section has-text-centered">
        <p className="title">Crux Counter Example</p>
        <p className="is-size-5">Rust Core, Rust Shell (Tauri + React.js)</p>
      </section>
      <section className="container has-text-centered">
        <p className="is-size-5">{view.text}</p>
        <div className="buttons section is-centered">
          <button
            className="button is-primary is-warning"
            onClick={() => {
              invoke("decrement");
            }}
          >
            {"Decrement"}
          </button>
          <button
            className="button is-primary is-danger"
            onClick={() => {
              invoke("increment");
            }}
          >
            {"Increment"}
          </button>
        </div>
      </section>
    </main>
  );
}

export default App;
