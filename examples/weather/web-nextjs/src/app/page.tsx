"use client";

import type { NextPage } from "next";
import { useEffect, useRef, useState } from "react";

import init_core from "shared/shared";
import {
  ViewModel,
  ViewModelVariantLoading,
  ViewModelVariantOnboard,
  ViewModelVariantActive,
  ViewModelVariantFailed,
  ActiveViewModelVariantHome,
  ActiveViewModelVariantFavorites,
  EventVariantStart,
} from "shared_types/app";

import { Core } from "../lib/core";
import { OnboardView } from "./views/OnboardView";
import { HomeView } from "./views/HomeView";
import { FavoritesView } from "./views/FavoritesView";

// Note: The generated types use class hierarchies to represent Rust enums,
// which requires `instanceof` checks throughout the view code. This will be
// made more idiomatic (discriminated unions) once
// https://github.com/redbadger/facet-generate/issues/83 is resolved.

// ANCHOR: content_view
const Home: NextPage = () => {
  const [view, setView] = useState(
    new ViewModelVariantLoading() as ViewModel,
  );
  const core: React.RefObject<Core | null> = useRef(null);

  const initialized = useRef(false);
  useEffect(
    () => {
      if (!initialized.current) {
        initialized.current = true;

        init_core().then(() => {
          if (core.current === null) {
            core.current = new Core(setView);
          }
          core.current?.update(new EventVariantStart());
        });
      }
    },
    /*once*/ [],
  );

  return (
    <main>
      <div className="app-container">
        <div className="app-header">
          <h1 className="title is-3">
            <i
              className="ph ph-cloud-sun"
              style={{ marginRight: "0.5rem" }}
            />
            Crux Weather
          </h1>
          <p className="subtitle is-6">
            Rust Core, TypeScript Shell (Next.js)
          </p>
        </div>
        {view instanceof ViewModelVariantLoading && (
          <div className="card">
            <div className="status-message">
              <i className="ph ph-spinner" />
              <p>Loading...</p>
            </div>
          </div>
        )}
        {view instanceof ViewModelVariantOnboard && (
          <OnboardView model={view.value} core={core} />
        )}
        {view instanceof ViewModelVariantActive && (
          <>
            {view.value instanceof ActiveViewModelVariantHome && (
              <HomeView model={view.value.value} core={core} />
            )}
            {view.value instanceof ActiveViewModelVariantFavorites && (
              <FavoritesView model={view.value.value} core={core} />
            )}
          </>
        )}
        {view instanceof ViewModelVariantFailed && (
          <div className="card">
            <div className="status-message">
              <i
                className="ph ph-warning-circle"
                style={{ color: "#ef4444" }}
              />
              <p style={{ color: "#ef4444" }}>{view.message}</p>
            </div>
          </div>
        )}
      </div>
    </main>
  );
};
// ANCHOR_END: content_view

export default Home;
