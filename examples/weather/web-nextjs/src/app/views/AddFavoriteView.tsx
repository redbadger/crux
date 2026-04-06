import { useState } from "react";

import type { Core } from "../../lib/core";
import type { AddFavoriteViewModel, AddFavoriteEvent } from "shared_types/app";
import {
  EventVariantActive,
  ActiveEventVariantFavorites,
  FavoritesScreenEventVariantWorkflow,
  FavoritesWorkflowEventVariantAdd,
  AddFavoriteEventVariantSearch,
  AddFavoriteEventVariantSubmit,
  AddFavoriteEventVariantCancel,
} from "shared_types/app";
import { SearchResultItem } from "./SearchResultItem";

function addFavEvent(inner: AddFavoriteEvent) {
  return new EventVariantActive(
    new ActiveEventVariantFavorites(
      new FavoritesScreenEventVariantWorkflow(
        new FavoritesWorkflowEventVariantAdd(inner),
      ),
    ),
  );
}

export function AddFavoriteView({
  model,
  core,
}: {
  model: AddFavoriteViewModel;
  core: React.RefObject<Core | null>;
}) {
  const [searchText, setSearchText] = useState(model.search_input || "");

  return (
    <>
      <div className="card">
        <p className="section-title">
          <i className="ph ph-magnifying-glass" />
          Add Favorite
        </p>
        <div className="field">
          <div className="control has-icons-left">
            <input
              className="input"
              type="text"
              placeholder="Search for a city..."
              value={searchText}
              onChange={(e) => {
                const val = e.target.value;
                setSearchText(val);
                if (val) {
                  core.current?.update(
                    addFavEvent(new AddFavoriteEventVariantSearch(val)),
                  );
                }
              }}
            />
            <span className="icon is-left">
              <i className="ph ph-magnifying-glass" />
            </span>
          </div>
        </div>
        {model.search_results && (
          <>
            {model.search_results.length === 0 ? (
              <div className="status-message">
                <i className="ph ph-map-pin-line" />
                <p>No results found</p>
              </div>
            ) : (
              model.search_results.map((result, i) => (
                <SearchResultItem
                  key={i}
                  result={result}
                  onAdd={() =>
                    core.current?.update(
                      addFavEvent(new AddFavoriteEventVariantSubmit(result)),
                    )
                  }
                />
              ))
            )}
          </>
        )}
      </div>
      <div className="buttons is-centered" style={{ marginTop: "1rem" }}>
        <button
          className="button btn"
          onClick={() =>
            core.current?.update(
              addFavEvent(new AddFavoriteEventVariantCancel()),
            )
          }
        >
          <span className="icon">
            <i className="ph ph-arrow-left" />
          </span>
          <span>Cancel</span>
        </button>
      </div>
    </>
  );
}
