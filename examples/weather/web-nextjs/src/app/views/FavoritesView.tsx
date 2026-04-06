import type { Core } from "../../lib/core";
import type { FavoritesViewModel, FavoritesScreenEvent } from "shared_types/app";
import {
  FavoritesWorkflowViewModelVariantAdd,
  FavoritesWorkflowViewModelVariantConfirmDelete,
  EventVariantActive,
  ActiveEventVariantFavorites,
  FavoritesScreenEventVariantGoToHome,
  FavoritesScreenEventVariantRequestAddFavorite,
  FavoritesScreenEventVariantRequestDelete,
  FavoritesScreenEventVariantWorkflow,
  FavoritesWorkflowEventVariantConfirmDelete,
  ConfirmDeleteEventVariantConfirmed,
  ConfirmDeleteEventVariantCancelled,
} from "shared_types/app";
import { AddFavoriteView } from "./AddFavoriteView";

function favEvent(inner: FavoritesScreenEvent) {
  return new EventVariantActive(new ActiveEventVariantFavorites(inner));
}

export function FavoritesView({
  model,
  core,
}: {
  model: FavoritesViewModel;
  core: React.RefObject<Core | null>;
}) {
  if (model.workflow instanceof FavoritesWorkflowViewModelVariantAdd) {
    return <AddFavoriteView model={model.workflow.value} core={core} />;
  }

  const deleteConfirmation =
    model.workflow instanceof FavoritesWorkflowViewModelVariantConfirmDelete
      ? model.workflow.location
      : null;

  return (
    <>
      <div className="card">
        <p className="section-title">
          <i className="ph ph-star" />
          Favorites
        </p>
        {model.favorites.length === 0 ? (
          <div className="status-message">
            <i className="ph ph-star" />
            <p>No favorites yet</p>
          </div>
        ) : (
          model.favorites.map((fav, i) => (
            <div
              key={i}
              className="fav-card"
              style={{ justifyContent: "space-between" }}
            >
              <span className="fav-name">
                <i
                  className="ph ph-map-pin"
                  style={{ marginRight: "0.25rem" }}
                />
                {fav.name}
              </span>
              <button
                className="button is-danger is-small btn"
                onClick={() =>
                  core.current?.update(
                    favEvent(
                      new FavoritesScreenEventVariantRequestDelete(
                        fav.location,
                      ),
                    ),
                  )
                }
              >
                <span className="icon is-small">
                  <i className="ph ph-trash" />
                </span>
              </button>
            </div>
          ))
        )}
      </div>
      {deleteConfirmation && (
        <div className="modal is-active">
          <div className="modal-background"></div>
          <div className="modal-content">
            <div className="card" style={{ textAlign: "center" }}>
              <i
                className="ph ph-warning"
                style={{ fontSize: "2.5rem", color: "#ef4444" }}
              />
              <p
                style={{
                  fontWeight: 600,
                  fontSize: "1.1rem",
                  margin: "0.75rem 0",
                }}
              >
                Delete this favorite?
              </p>
              <div className="buttons is-centered">
                <button
                  className="button btn"
                  onClick={() =>
                    core.current?.update(
                      favEvent(
                        new FavoritesScreenEventVariantWorkflow(
                          new FavoritesWorkflowEventVariantConfirmDelete(
                            new ConfirmDeleteEventVariantCancelled(),
                          ),
                        ),
                      ),
                    )
                  }
                >
                  Cancel
                </button>
                <button
                  className="button is-danger btn"
                  onClick={() =>
                    core.current?.update(
                      favEvent(
                        new FavoritesScreenEventVariantWorkflow(
                          new FavoritesWorkflowEventVariantConfirmDelete(
                            new ConfirmDeleteEventVariantConfirmed(),
                          ),
                        ),
                      ),
                    )
                  }
                >
                  <span className="icon">
                    <i className="ph ph-trash" />
                  </span>
                  <span>Delete</span>
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
      <div className="buttons is-centered" style={{ marginTop: "1rem" }}>
        <button
          className="button btn"
          onClick={() =>
            core.current?.update(
              favEvent(new FavoritesScreenEventVariantGoToHome()),
            )
          }
        >
          <span className="icon">
            <i className="ph ph-arrow-left" />
          </span>
          <span>Back</span>
        </button>
        <button
          className="button is-primary btn"
          onClick={() =>
            core.current?.update(
              favEvent(new FavoritesScreenEventVariantRequestAddFavorite()),
            )
          }
        >
          <span className="icon">
            <i className="ph ph-plus" />
          </span>
          <span>Add Favorite</span>
        </button>
      </div>
    </>
  );
}
