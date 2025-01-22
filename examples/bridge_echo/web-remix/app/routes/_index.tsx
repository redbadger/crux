import { useEffect, useState } from "react";

import {
  ViewModel,
  EventVariantTick,
  EventVariantNewPeriod,
} from "shared_types/types/shared_types";
import { update } from "../core";

export const meta = () => {
  return [
    { title: "New Remix App" },
    { name: "description", content: "Welcome to Remix!" },
  ];
};

export default function Index() {
  const [view, setView] = useState(new ViewModel(BigInt(0), []));

  // Tick as fast as we can
  useEffect(() => {
    var run = true;
    const tick = () => {
      update(new EventVariantTick(), setView);

      if (run) {
        setTimeout(tick, 0);
      }
    };

    tick();

    return () => {
      run = false;
    };
  }, []);

  // Once a second reset the period
  useEffect(() => {
    const id = setInterval(() => {
      update(new EventVariantNewPeriod(), setView);
    }, 1000);

    return () => {
      clearInterval(id);
    };
  }, []);

  return (
    <main>
      <section className="box container has-text-centered m-5">
        <p className="is-size-5">{view.count.toString()}</p>
      </section>
    </main>
  );
}
