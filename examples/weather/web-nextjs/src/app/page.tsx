"use client";

import { useMemo } from "react";
import type { NextPage } from "next";
import { CloudSun, WarningCircle } from "@phosphor-icons/react";

import type {
  ActiveViewModel,
  FavoritesViewModel,
  HomeViewModel,
  OnboardViewModel,
  ViewModel,
} from "shared_types/app";
import {
  ViewModelVariantLoading,
  ViewModelVariantOnboard,
  ViewModelVariantActive,
  ViewModelVariantFailed,
  ActiveViewModelVariantHome,
  ActiveViewModelVariantFavorites,
} from "shared_types/app";

import { CoreProvider, useViewModel } from "../lib/core/provider";
import {
  Card,
  ScreenHeader,
  Spinner,
  StatusMessage,
} from "./components/common";
import { OnboardView } from "./components/OnboardView";
import { HomeView } from "./components/HomeView";
import { FavoritesView } from "./components/FavoritesView";

// Note: The generated types use class hierarchies to represent Rust enums,
// which requires `instanceof` checks throughout the view code. This will be
// made more idiomatic (discriminated unions) once
// https://github.com/redbadger/facet-generate/issues/83 is resolved.

// ANCHOR: app
const AppShell = () => {
  const view = useViewModel();

  // Project the top-level view model into per-stage slices. React's useMemo
  // is the coarse equivalent of Leptos's `Memo`: recomputes only when `view`
  // changes, but doesn't diff deeper than reference equality.
  const onboardVm = useMemo(
    () => (view instanceof ViewModelVariantOnboard ? view.value : null),
    [view],
  );
  const homeVm = useMemo(() => pickHome(view), [view]);
  const favoritesVm = useMemo(() => pickFavorites(view), [view]);
  const failedMessage = useMemo(
    () => (view instanceof ViewModelVariantFailed ? view.message : null),
    [view],
  );

  return (
    <main className="max-w-xl mx-auto px-4 py-8">
      <ScreenHeader
        title="Crux Weather"
        subtitle="Rust Core, TypeScript Shell (Next.js)"
        icon={CloudSun}
      />
      {view instanceof ViewModelVariantLoading && (
        <Card>
          <Spinner message="Loading..." />
        </Card>
      )}
      {onboardVm && <OnboardView model={onboardVm} />}
      {homeVm && <HomeView model={homeVm} />}
      {favoritesVm && <FavoritesView model={favoritesVm} />}
      {failedMessage !== null && (
        <Card>
          <StatusMessage
            icon={WarningCircle}
            message={failedMessage}
            tone="error"
          />
        </Card>
      )}
    </main>
  );
};
// ANCHOR_END: app

function pickHome(view: ViewModel): HomeViewModel | null {
  if (!(view instanceof ViewModelVariantActive)) return null;
  const active: ActiveViewModel = view.value;
  if (!(active instanceof ActiveViewModelVariantHome)) return null;
  return active.value;
}

function pickFavorites(view: ViewModel): FavoritesViewModel | null {
  if (!(view instanceof ViewModelVariantActive)) return null;
  const active: ActiveViewModel = view.value;
  if (!(active instanceof ActiveViewModelVariantFavorites)) return null;
  return active.value;
}

const Home: NextPage = () => (
  <CoreProvider>
    <AppShell />
  </CoreProvider>
);

export default Home;
