import { useEffect, useRef, useState } from "react";

import {
  ViewModel,
  EventVariantTick,
  EventVariantNewPeriod,
  DataPoint,
} from "app/app";
import { Core } from "../core";

export const meta = () => {
  return [
    { title: "New Remix App" },
    { name: "description", content: "Welcome to Remix!" },
  ];
};

export default function Index() {
  const [view, setView] = useState(new ViewModel(BigInt(0), [], []));

  const core: React.RefObject<Core | null> = useRef(null);
  useEffect(
    () => {
      // There may be a nicer way using https://react.dev/reference/react/useSyncExternalStore
      if (core.current === null) {
        core.current = new Core(setView);
      }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    /*once*/ []
  );

  const payload: DataPoint[] = Array(10)
    .fill(null)
    .map((_, idx) => {
      const add_meta: boolean = Math.random() > 0.5;
      const payload = new DataPoint(
        BigInt(idx),
        idx,
        `item_${idx}`,
        add_meta ? `meta_${idx}` : null
      );

      return payload;
    });

  // Tick as fast as we can
  useEffect(() => {
    var run = true;
    const tick = () => {
      core.current?.update(new EventVariantTick(payload));

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
      core.current?.update(new EventVariantNewPeriod());
    }, 1000);

    return () => {
      clearInterval(id);
    };
  }, []);

  return (
    <main>
      <section className="box container has-text-centered m-5">
        <p className="is-size-5">{view.count.toString()}</p>
        <p className="is-size-5">overall average: {view.average}</p>
        <p className="is-size-5">
          10 second moving average: {view.moving_average}
        </p>
        <p className="is-size-5">max: {view.max}</p>
      </section>
    </main>
  );
}
