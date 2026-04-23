import {
  ArrowCounterClockwise,
  Check,
  Key,
  Warning,
  type Icon as PhosphorIcon,
} from "@phosphor-icons/react";

import type { OnboardViewModel, OnboardReason } from "shared_types/app";
import {
  OnboardReasonVariantWelcome,
  OnboardReasonVariantUnauthorized,
  OnboardReasonVariantReset,
  OnboardStateViewModelVariantInput,
  OnboardStateViewModelVariantSaving,
  EventVariantOnboard,
  OnboardEventVariantApiKey,
  OnboardEventVariantSubmit,
} from "shared_types/app";

import { useDispatch } from "../../lib/core/provider";
import {
  Button,
  Card,
  SectionTitle,
  Spinner,
  TextField,
} from "./common";

// ANCHOR: onboard_view
export function OnboardView({ model }: { model: OnboardViewModel }) {
  const dispatch = useDispatch();
  const { icon, reasonText } = reasonCopy(model.reason);

  if (model.state instanceof OnboardStateViewModelVariantInput) {
    const { api_key, can_submit } = model.state;
    return (
      <Card>
        <SectionTitle icon={icon} title="Setup" />
        <p className="text-slate-500 text-sm mb-4">{reasonText}</p>
        <div className="mb-4">
          <TextField
            value={api_key}
            placeholder="Paste your API key here"
            icon={Key}
            onInput={(value) =>
              dispatch(
                new EventVariantOnboard(new OnboardEventVariantApiKey(value)),
              )
            }
          />
        </div>
        <Button
          label="Submit"
          icon={Check}
          enabled={can_submit}
          fullWidth
          onClick={() =>
            dispatch(new EventVariantOnboard(new OnboardEventVariantSubmit()))
          }
        />
      </Card>
    );
  }

  if (model.state instanceof OnboardStateViewModelVariantSaving) {
    return (
      <Card>
        <Spinner message="Saving..." />
      </Card>
    );
  }

  return null;
}
// ANCHOR_END: onboard_view

function reasonCopy(reason: OnboardReason): {
  icon: PhosphorIcon;
  reasonText: string;
} {
  if (reason instanceof OnboardReasonVariantWelcome) {
    return {
      icon: Key,
      reasonText: "Welcome! Enter your OpenWeather API key to get started.",
    };
  }
  if (reason instanceof OnboardReasonVariantUnauthorized) {
    return {
      icon: Warning,
      reasonText: "Your API key was rejected. Please enter a valid key.",
    };
  }
  if (reason instanceof OnboardReasonVariantReset) {
    return {
      icon: ArrowCounterClockwise,
      reasonText: "Enter a new API key.",
    };
  }
  return { icon: Key, reasonText: "" };
}
