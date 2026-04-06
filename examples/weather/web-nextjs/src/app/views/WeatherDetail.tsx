import type { CurrentWeatherResponse } from "shared_types/app";

export function WeatherDetail({ data }: { data: CurrentWeatherResponse }) {
  const desc = data.weather?.[0]?.description;
  return (
    <div style={{ textAlign: "center" }}>
      <p
        style={{
          fontWeight: 600,
          color: "#6b7280",
          fontSize: "0.9rem",
          textTransform: "uppercase",
          letterSpacing: "0.05em",
        }}
      >
        <i className="ph ph-map-pin" style={{ marginRight: "0.25rem" }} />
        {data.name}
      </p>
      <p className="temp-large">{data.main.temp.toFixed(1)}&deg;</p>
      {desc && <p className="weather-desc">{desc}</p>}
      <div className="stat-grid">
        <div>
          <p className="stat-label">
            <i
              className="ph ph-thermometer"
              style={{ marginRight: "0.2rem" }}
            />
            Feels Like
          </p>
          <p className="stat-value">
            {data.main.feels_like.toFixed(1)}&deg;
          </p>
        </div>
        <div>
          <p className="stat-label">
            <i className="ph ph-drop" style={{ marginRight: "0.2rem" }} />
            Humidity
          </p>
          <p className="stat-value">{Number(data.main.humidity)}%</p>
        </div>
        <div>
          <p className="stat-label">
            <i className="ph ph-wind" style={{ marginRight: "0.2rem" }} />
            Wind
          </p>
          <p className="stat-value">{data.wind.speed.toFixed(1)} m/s</p>
        </div>
        <div>
          <p className="stat-label">
            <i className="ph ph-gauge" style={{ marginRight: "0.2rem" }} />
            Pressure
          </p>
          <p className="stat-value">{Number(data.main.pressure)} hPa</p>
        </div>
        <div>
          <p className="stat-label">
            <i className="ph ph-cloud" style={{ marginRight: "0.2rem" }} />
            Clouds
          </p>
          <p className="stat-value">{Number(data.clouds.all)}%</p>
        </div>
        <div>
          <p className="stat-label">
            <i className="ph ph-eye" style={{ marginRight: "0.2rem" }} />
            Visibility
          </p>
          <p className="stat-value">
            {Math.floor(Number(data.visibility) / 1000)} km
          </p>
        </div>
      </div>
    </div>
  );
}
