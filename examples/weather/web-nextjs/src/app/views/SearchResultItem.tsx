import type { GeocodingResponse } from "shared_types/app";

export function SearchResultItem({
  result,
  onAdd,
}: {
  result: GeocodingResponse;
  onAdd: () => void;
}) {
  return (
    <div className="fav-card" style={{ justifyContent: "space-between" }}>
      <div>
        <span className="fav-name">
          <i
            className="ph ph-map-pin"
            style={{ marginRight: "0.25rem" }}
          />
          {result.name}
        </span>
        <br />
        <small style={{ color: "#9ca3af" }}>
          {result.state
            ? `${result.state}, ${result.country}`
            : result.country}
        </small>
      </div>
      <button className="button is-primary is-small btn" onClick={onAdd}>
        <span className="icon is-small">
          <i className="ph ph-plus" />
        </span>
        <span>Add</span>
      </button>
    </div>
  );
}
