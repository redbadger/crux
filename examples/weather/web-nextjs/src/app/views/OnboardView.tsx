import type { Core } from "../../lib/core";
import type { OnboardViewModel } from "shared_types/app";
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

export function OnboardView({
  model,
  core,
}: {
  model: OnboardViewModel;
  core: React.RefObject<Core | null>;
}) {
  let icon: string;
  let reasonText: string;

  if (model.reason instanceof OnboardReasonVariantWelcome) {
    icon = "ph ph-key";
    reasonText = "Welcome! Enter your OpenWeather API key to get started.";
  } else if (model.reason instanceof OnboardReasonVariantUnauthorized) {
    icon = "ph ph-warning";
    reasonText = "Your API key was rejected. Please enter a valid key.";
  } else if (model.reason instanceof OnboardReasonVariantReset) {
    icon = "ph ph-arrow-counter-clockwise";
    reasonText = "Enter a new API key.";
  } else {
    icon = "ph ph-key";
    reasonText = "";
  }

  if (model.state instanceof OnboardStateViewModelVariantInput) {
    const { api_key, can_submit } = model.state;
    return (
      <div className="card">
        <p className="section-title">
          <i className={icon} />
          Setup
        </p>
        <p style={{ color: "#6b7280", marginBottom: "1rem" }}>{reasonText}</p>
        <div className="field">
          <div className="control has-icons-left">
            <input
              className="input"
              type="text"
              placeholder="Paste your API key here"
              value={api_key}
              onChange={(e) =>
                core.current?.update(
                  new EventVariantOnboard(
                    new OnboardEventVariantApiKey(e.target.value),
                  ),
                )
              }
            />
            <span className="icon is-left">
              <i className="ph ph-key" />
            </span>
          </div>
        </div>
        <button
          className="button is-primary btn"
          disabled={!can_submit}
          onClick={() =>
            core.current?.update(
              new EventVariantOnboard(new OnboardEventVariantSubmit()),
            )
          }
        >
          <span className="icon">
            <i className="ph ph-check" />
          </span>
          <span>Submit</span>
        </button>
      </div>
    );
  }

  if (model.state instanceof OnboardStateViewModelVariantSaving) {
    return (
      <div className="card">
        <div className="status-message">
          <i className="ph ph-spinner" />
          <p>Saving...</p>
        </div>
      </div>
    );
  }

  return null;
}
