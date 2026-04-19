import { useState } from "react";
import { ArrowLeft, MagnifyingGlass, MapPinLine } from "@phosphor-icons/react";

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

import { useDispatch } from "../../lib/core/provider";
import { Button, Card, SectionTitle, StatusMessage, TextField } from "./common";
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

export function AddFavoriteView({ model }: { model: AddFavoriteViewModel }) {
  const dispatch = useDispatch();
  const [searchText, setSearchText] = useState(model.search_input || "");

  return (
    <>
      <Card className="mb-4">
        <SectionTitle icon={MagnifyingGlass} title="Add Favourite" />
        <div className="mb-4">
          <TextField
            value={searchText}
            placeholder="Search for a city..."
            icon={MagnifyingGlass}
            onInput={(val) => {
              setSearchText(val);
              if (val) {
                dispatch(addFavEvent(new AddFavoriteEventVariantSearch(val)));
              }
            }}
          />
        </div>
        {model.search_results &&
          (model.search_results.length === 0 ? (
            <StatusMessage icon={MapPinLine} message="No results found" />
          ) : (
            <div className="grid gap-2">
              {model.search_results.map((result, i) => (
                <SearchResultItem
                  key={i}
                  result={result}
                  onAdd={() =>
                    dispatch(
                      addFavEvent(new AddFavoriteEventVariantSubmit(result)),
                    )
                  }
                />
              ))}
            </div>
          ))}
      </Card>
      <div className="flex justify-center gap-2 mt-4">
        <Button
          label="Cancel"
          icon={ArrowLeft}
          variant="secondary"
          onClick={() =>
            dispatch(addFavEvent(new AddFavoriteEventVariantCancel()))
          }
        />
      </div>
    </>
  );
}
