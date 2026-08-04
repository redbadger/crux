import {
  ArrowCounterClockwise,
  Check,
  Key,
  Warning,
  type Icon as PhosphorIcon,
} from "@phosphor-icons/react";

import type { OnboardViewModel, OnboardReason } from "shared_types/app";
import {
  matchOnboardReason,
  matchOnboardStateViewModel,
  eventOnboard,
  onboardEventApiKey,
  onboardEventSubmit,
} from "shared_types/app";

import { useDispatch } from "../../lib/core/provider";
import { Button, Card, SectionTitle, Spinner, TextField } from "./common";

// ANCHOR: onboard_view
export function OnboardView({ model }: { model: OnboardViewModel }) {
  const dispatch = useDispatch();
  const { icon, reasonText } = reasonCopy(model.reason);

  return matchOnboardStateViewModel(model.state, {
    Input: (s) => (
      <Card>
        <SectionTitle icon={icon} title="Setup" />
        <p className="text-slate-500 text-sm mb-4">{reasonText}</p>
        <div className="mb-4">
          <TextField
            value={s.api_key}
            placeholder="Paste your API key here"
            icon={Key}
            onInput={(value) =>
              dispatch(eventOnboard(onboardEventApiKey(value)))
            }
          />
        </div>
        <Button
          label="Submit"
          icon={Check}
          enabled={s.can_submit}
          fullWidth
          onClick={() => dispatch(eventOnboard(onboardEventSubmit()))}
        />
      </Card>
    ),
    Saving: () => (
      <Card>
        <Spinner message="Saving..." />
      </Card>
    ),
  });
}
// ANCHOR_END: onboard_view

function reasonCopy(reason: OnboardReason): {
  icon: PhosphorIcon;
  reasonText: string;
} {
  return matchOnboardReason(reason, {
    Welcome: () => ({
      icon: Key,
      reasonText: "Welcome! Enter your OpenWeather API key to get started.",
    }),
    Unauthorized: () => ({
      icon: Warning,
      reasonText: "Your API key was rejected. Please enter a valid key.",
    }),
    Reset: () => ({
      icon: ArrowCounterClockwise,
      reasonText: "Enter a new API key.",
    }),
  });
}
