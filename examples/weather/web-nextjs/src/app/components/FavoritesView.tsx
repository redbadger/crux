import {
  ArrowLeft,
  MapPin,
  Plus,
  Star,
  Trash,
  Warning,
} from "@phosphor-icons/react";

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

import { useDispatch } from "../../lib/core/provider";
import { AddFavoriteView } from "./AddFavoriteView";
import {
  Button,
  Card,
  IconButton,
  Modal,
  SectionTitle,
  StatusMessage,
} from "./common";

function favEvent(inner: FavoritesScreenEvent) {
  return new EventVariantActive(new ActiveEventVariantFavorites(inner));
}

export function FavoritesView({ model }: { model: FavoritesViewModel }) {
  const dispatch = useDispatch();

  if (model.workflow instanceof FavoritesWorkflowViewModelVariantAdd) {
    return <AddFavoriteView model={model.workflow.value} />;
  }

  const isConfirmingDelete =
    model.workflow instanceof FavoritesWorkflowViewModelVariantConfirmDelete;

  return (
    <>
      <Card className="mb-4">
        <SectionTitle icon={Star} title="Favourites" />
        {model.favorites.length === 0 ? (
          <StatusMessage icon={Star} message="No favourites yet" />
        ) : (
          <div className="grid gap-2">
            {model.favorites.map((fav, i) => (
              <div
                key={i}
                className="bg-slate-50 rounded-xl px-4 py-3 flex items-center justify-between gap-4"
              >
                <span className="font-semibold text-slate-900 flex items-center gap-1">
                  <MapPin size={16} />
                  {fav.name}
                </span>
                <IconButton
                  icon={Trash}
                  variant="danger"
                  ariaLabel="Delete favourite"
                  onClick={() =>
                    dispatch(
                      favEvent(
                        new FavoritesScreenEventVariantRequestDelete(
                          fav.location,
                        ),
                      ),
                    )
                  }
                />
              </div>
            ))}
          </div>
        )}
      </Card>
      {isConfirmingDelete && (
        <Modal>
          <Card>
            <div className="flex flex-col items-center text-center gap-4">
              <span className="text-red-500">
                <Warning size={40} />
              </span>
              <p className="text-slate-900 font-semibold text-lg">
                Delete this favourite?
              </p>
              <div className="flex gap-2">
                <Button
                  label="Cancel"
                  variant="secondary"
                  onClick={() =>
                    dispatch(
                      favEvent(
                        new FavoritesScreenEventVariantWorkflow(
                          new FavoritesWorkflowEventVariantConfirmDelete(
                            new ConfirmDeleteEventVariantCancelled(),
                          ),
                        ),
                      ),
                    )
                  }
                />
                <Button
                  label="Delete"
                  icon={Trash}
                  variant="danger"
                  onClick={() =>
                    dispatch(
                      favEvent(
                        new FavoritesScreenEventVariantWorkflow(
                          new FavoritesWorkflowEventVariantConfirmDelete(
                            new ConfirmDeleteEventVariantConfirmed(),
                          ),
                        ),
                      ),
                    )
                  }
                />
              </div>
            </div>
          </Card>
        </Modal>
      )}
      <div className="flex justify-center gap-2 mt-4">
        <Button
          label="Back"
          icon={ArrowLeft}
          variant="secondary"
          onClick={() =>
            dispatch(favEvent(new FavoritesScreenEventVariantGoToHome()))
          }
        />
        <Button
          label="Add Favourite"
          icon={Plus}
          onClick={() =>
            dispatch(
              favEvent(new FavoritesScreenEventVariantRequestAddFavorite()),
            )
          }
        />
      </div>
    </>
  );
}
