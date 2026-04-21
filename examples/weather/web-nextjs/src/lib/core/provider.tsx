"use client";

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";

import init_core from "shared/shared";
import type { Event, ViewModel } from "shared_types/app";
import { EventVariantStart, ViewModelVariantLoading } from "shared_types/app";

import { Core } from "./";

// ANCHOR: context
/**
 * Two separate contexts so components that only need `dispatch` don't
 * re-render when the view model changes. Mirrors the Leptos split between
 * `Signal<ViewModel>` and `UnsyncCallback<Event>`.
 */
const ViewModelContext = createContext<ViewModel | null>(null);
const DispatchContext = createContext<((event: Event) => void) | null>(null);
// ANCHOR_END: context

// ANCHOR: provider
/**
 * Creates the Crux core once, drives its view model into React state, and
 * exposes `dispatch` as a stable callback via context.
 */
export function CoreProvider({ children }: { children: ReactNode }) {
  const [view, setView] = useState<ViewModel>(new ViewModelVariantLoading());
  const coreRef = useRef<Core | null>(null);
  const initialized = useRef(false);

  useEffect(() => {
    if (initialized.current) return;
    initialized.current = true;

    init_core().then(() => {
      if (!coreRef.current) {
        coreRef.current = new Core(setView);
      }
      coreRef.current.update(new EventVariantStart());
    });
  }, []);

  // Stable across renders — `useCallback` with empty deps keeps the same
  // reference, so context consumers don't cascade unnecessary renders.
  const dispatch = useCallback((event: Event) => {
    coreRef.current?.update(event);
  }, []);

  return (
    <DispatchContext.Provider value={dispatch}>
      <ViewModelContext.Provider value={view}>
        {children}
      </ViewModelContext.Provider>
    </DispatchContext.Provider>
  );
}
// ANCHOR_END: provider

// ANCHOR: hooks
export function useViewModel(): ViewModel {
  const view = useContext(ViewModelContext);
  if (view === null) {
    throw new Error("useViewModel must be used within CoreProvider");
  }
  return view;
}

export function useDispatch(): (event: Event) => void {
  const dispatch = useContext(DispatchContext);
  if (dispatch === null) {
    throw new Error("useDispatch must be used within CoreProvider");
  }
  return dispatch;
}
// ANCHOR_END: hooks
