import {
  Cloud,
  Drop,
  Eye,
  Gauge,
  MapPin,
  Thermometer,
  Wind,
  type Icon as PhosphorIcon,
} from "@phosphor-icons/react";

import type { CurrentWeatherResponse } from "shared_types/app";

export function WeatherDetail({ data }: { data: CurrentWeatherResponse }) {
  const desc = data.weather?.[0]?.description;
  return (
    <div className="text-center">
      <p className="uppercase tracking-wide text-xs text-slate-500 font-semibold flex items-center justify-center gap-1">
        <MapPin size={14} />
        {data.name}
      </p>
      <p className="text-6xl font-bold text-slate-900 mt-2 leading-none">
        {data.main.temp.toFixed(1)}&deg;
      </p>
      {desc ? (
        <p className="text-slate-500 mt-1 capitalize">{desc}</p>
      ) : null}
      <div className="grid grid-cols-3 gap-3 mt-6 text-center">
        <Stat
          icon={Thermometer}
          label="Feels Like"
          value={`${data.main.feels_like.toFixed(1)}\u00b0`}
        />
        <Stat
          icon={Drop}
          label="Humidity"
          value={`${Number(data.main.humidity)}%`}
        />
        <Stat
          icon={Wind}
          label="Wind"
          value={`${data.wind.speed.toFixed(1)} m/s`}
        />
        <Stat
          icon={Gauge}
          label="Pressure"
          value={`${Number(data.main.pressure)} hPa`}
        />
        <Stat
          icon={Cloud}
          label="Clouds"
          value={`${Number(data.clouds.all)}%`}
        />
        <Stat
          icon={Eye}
          label="Visibility"
          value={`${Math.floor(Number(data.visibility) / 1000)} km`}
        />
      </div>
    </div>
  );
}

function Stat({
  icon: Icon,
  label,
  value,
}: {
  icon: PhosphorIcon;
  label: string;
  value: string;
}) {
  return (
    <div>
      <p className="text-xs uppercase tracking-wide text-slate-400 font-semibold flex items-center justify-center gap-1">
        <Icon size={14} />
        {label}
      </p>
      <p className="text-sm font-semibold text-slate-700 mt-1">{value}</p>
    </div>
  );
}
