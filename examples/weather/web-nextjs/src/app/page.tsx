"use client";

import type { NextPage } from "next";
import { useEffect, useRef, useState } from "react";

import init_core from "shared/shared";
import {
  ViewModel,
  WorkflowViewModelVariantHome,
  WorkflowViewModelVariantFavorites,
  WorkflowViewModelVariantAddFavorite,
  EventVariantHome,
  EventVariantNavigate,
  EventVariantFavorites,
  WeatherEventVariantShow,
  WorkflowVariantHome,
  WorkflowVariantFavorites,
  WorkflowVariantAddFavorite,
  FavoritesStateVariantIdle,
  FavoritesEventVariantDeletePressed,
  FavoritesEventVariantDeleteConfirmed,
  FavoritesEventVariantDeleteCancelled,
  FavoritesEventVariantSearch,
  FavoritesEventVariantSubmit,
  FavoritesEventVariantRestore,
} from "shared_types/app";

import { Core } from "./core";

// ANCHOR: content_view
const Home: NextPage = () => {
  const [view, setView] = useState(
    new ViewModel(new WorkflowViewModelVariantHome(null!, [])),
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
          core.current?.update(
            new EventVariantHome(new WeatherEventVariantShow()),
          );
        });
      }
    },
     
    /*once*/ [],
  );

  const workflow = view.workflow;

  return (
    <main>
      <section className="section has-text-centered">
        <p className="title">Crux Weather Example</p>
        <p className="is-size-5">Rust Core, TypeScript Shell (Next.js)</p>
      </section>
      <section className="container">
        {workflow instanceof WorkflowViewModelVariantHome && (
          <HomeView
            weatherData={workflow.weather_data}
            favorites={workflow.favorites}
            core={core}
          />
        )}
        {workflow instanceof WorkflowViewModelVariantFavorites && (
          <FavoritesView
            favorites={workflow.favorites}
            deleteConfirmation={workflow.delete_confirmation}
            core={core}
          />
        )}
        {workflow instanceof WorkflowViewModelVariantAddFavorite && (
          <AddFavoriteView searchResults={workflow.search_results} core={core} />
        )}
      </section>
    </main>
  );
};
// ANCHOR_END: content_view

// ANCHOR: home_view
function HomeView({
  weatherData,
  favorites,
  core,
}: {
  weatherData: unknown;
  favorites: unknown[];
  core: React.RefObject<Core | null>;
}) {
   
  const wd = weatherData as any;
  const hasData = wd && wd.cod == 200;
   
  const favs = favorites as any[];

  return (
    <>
      <div className="box">
        {hasData ? (
          <div className="has-text-centered">
            <h2 className="title is-4">{wd.name}</h2>
            <p className="is-size-1 has-text-weight-bold">
              {wd.main.temp.toFixed(1)}&deg;
            </p>
            {wd.weather?.[0] && (
              <p className="is-size-5">{wd.weather[0].description}</p>
            )}
            <div className="columns is-multiline is-centered mt-4">
              <div className="column is-one-third">
                <p className="heading">Feels Like</p>
                <p>{wd.main.feels_like.toFixed(1)}&deg;</p>
              </div>
              <div className="column is-one-third">
                <p className="heading">Humidity</p>
                <p>{Number(wd.main.humidity)}%</p>
              </div>
              <div className="column is-one-third">
                <p className="heading">Wind</p>
                <p>{wd.wind.speed.toFixed(1)} m/s</p>
              </div>
              <div className="column is-one-third">
                <p className="heading">Pressure</p>
                <p>{Number(wd.main.pressure)} hPa</p>
              </div>
              <div className="column is-one-third">
                <p className="heading">Clouds</p>
                <p>{Number(wd.clouds.all)}%</p>
              </div>
              <div className="column is-one-third">
                <p className="heading">Visibility</p>
                <p>{Math.floor(Number(wd.visibility) / 1000)} km</p>
              </div>
            </div>
          </div>
        ) : (
          <p className="has-text-centered">Loading weather data...</p>
        )}
      </div>
      {favs.length > 0 && (
        <div className="box">
          <h3 className="title is-5">Favorites</h3>
          {favs.map((fav, i) => {
            const w = fav.current;
            return (
              <div key={i} className="box">
                <strong>{fav.name}</strong>
                {w ? (
                  <div className="columns is-multiline mt-2">
                    <div className="column is-one-third">
                      <p className="is-size-3 has-text-weight-bold">
                        {w.main.temp.toFixed(1)}&deg;
                      </p>
                    </div>
                    <div className="column is-one-third">
                      {w.weather?.[0] && <p>{w.weather[0].description}</p>}
                    </div>
                    <div className="column is-one-third">
                      <p>Humidity: {Number(w.main.humidity)}%</p>
                    </div>
                  </div>
                ) : (
                  <p className="has-text-grey">Loading...</p>
                )}
              </div>
            );
          })}
        </div>
      )}
      <div className="buttons is-centered mt-4">
        <button
          className="button is-info"
          onClick={() =>
            core.current?.update(
              new EventVariantNavigate(
                new WorkflowVariantFavorites(new FavoritesStateVariantIdle()),
              ),
            )
          }
        >
          Favorites
        </button>
      </div>
    </>
  );
}
// ANCHOR_END: home_view

function FavoritesView({
  favorites,
  deleteConfirmation,
  core,
}: {
  favorites: unknown[];
  deleteConfirmation: unknown;
  core: React.RefObject<Core | null>;
}) {
   
  const favs = favorites as any[];

  useEffect(() => {
    core.current?.update(
      new EventVariantFavorites(new FavoritesEventVariantRestore()),
    );
  }, [core]);

  return (
    <>
      <div className="box">
        <h2 className="title is-4">Favorites</h2>
        {favs.length === 0 ? (
          <p>No favorites yet</p>
        ) : (
          favs.map((fav, i) => (
            <div key={i} className="box level">
              <div className="level-left">
                <strong>{fav.name}</strong>
              </div>
              <div className="level-right">
                <button
                  className="button is-danger is-small"
                  onClick={() =>
                    core.current?.update(
                      new EventVariantFavorites(
                        new FavoritesEventVariantDeletePressed(fav.location),
                      ),
                    )
                  }
                >
                  Delete
                </button>
              </div>
            </div>
          ))
        )}
      </div>
      {deleteConfirmation && (
        <div className="modal is-active">
          <div className="modal-background"></div>
          <div className="modal-content">
            <div className="box has-text-centered">
              <p className="title is-5">Delete Favorite?</p>
              <div className="buttons is-centered">
                <button
                  className="button"
                  onClick={() =>
                    core.current?.update(
                      new EventVariantFavorites(
                        new FavoritesEventVariantDeleteCancelled(),
                      ),
                    )
                  }
                >
                  Cancel
                </button>
                <button
                  className="button is-danger"
                  onClick={() =>
                    core.current?.update(
                      new EventVariantFavorites(
                        new FavoritesEventVariantDeleteConfirmed(),
                      ),
                    )
                  }
                >
                  Delete
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
      <div className="buttons is-centered mt-4">
        <button
          className="button"
          onClick={() =>
            core.current?.update(
              new EventVariantNavigate(new WorkflowVariantHome()),
            )
          }
        >
          Back
        </button>
        <button
          className="button is-primary"
          onClick={() =>
            core.current?.update(
              new EventVariantNavigate(new WorkflowVariantAddFavorite()),
            )
          }
        >
          Add Favorite
        </button>
      </div>
    </>
  );
}

function AddFavoriteView({
  searchResults,
  core,
}: {
  searchResults: unknown[] | null;
  core: React.RefObject<Core | null>;
}) {
  const [searchText, setSearchText] = useState("");
   
  const results = searchResults as any[] | null;

  return (
    <>
      <div className="box">
        <h2 className="title is-4">Add Favorite</h2>
        <div className="field has-addons">
          <div className="control is-expanded">
            <input
              className="input"
              type="text"
              placeholder="Search location..."
              value={searchText}
              onChange={(e) => {
                const val = e.target.value;
                setSearchText(val);
                if (val) {
                  core.current?.update(
                    new EventVariantFavorites(
                      new FavoritesEventVariantSearch(val),
                    ),
                  );
                }
              }}
            />
          </div>
        </div>
        {results && (
          <>
            {results.length === 0 ? (
              <p>No results found</p>
            ) : (
              results.map((result, i) => (
                <div key={i} className="box">
                  <div className="level">
                    <div className="level-left">
                      <div>
                        <strong>{result.name}</strong>
                        <br />
                        <small>
                          {result.state
                            ? `${result.state}, ${result.country}`
                            : result.country}
                        </small>
                      </div>
                    </div>
                    <div className="level-right">
                      <button
                        className="button is-primary is-small"
                        onClick={() =>
                          core.current?.update(
                            new EventVariantFavorites(
                              new FavoritesEventVariantSubmit(result),
                            ),
                          )
                        }
                      >
                        Add
                      </button>
                    </div>
                  </div>
                </div>
              ))
            )}
          </>
        )}
      </div>
      <div className="buttons is-centered mt-4">
        <button
          className="button"
          onClick={() =>
            core.current?.update(
              new EventVariantNavigate(new WorkflowVariantHome()),
            )
          }
        >
          Cancel
        </button>
      </div>
    </>
  );
}

export default Home;
