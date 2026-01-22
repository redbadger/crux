package com.crux.example.weather.ui.home

import androidx.compose.runtime.Immutable

@Immutable
data class HomeUiState(
    val pages: List<HomePageUi>,
)

sealed interface HomePageUi {
    data class Weather(val card: WeatherCardUi) : HomePageUi
    object Loading : HomePageUi
}

@Immutable
data class WeatherCardUi(
    val title: String,
    val temperature: Double,
    val condition: String?,
    val description: String?,
    val tempMin: Double,
    val tempMax: Double,
    val details: List<WeatherDetailUi>,
    val sunrise: Long,
    val sunset: Long,
    val weatherType: WeatherType,
)

data class WeatherDetailUi(
    val type: WeatherDetailType,
    val value: Double,
)

enum class WeatherType {
    Thunderstorm,
    Drizzle,
    Rain,
    Snow,
    Atmosphere,
    ClearDay,
    ClearNight,
    Clouds,
    Unknown,
}

enum class WeatherDetailType {
    FeelsLike,
    Humidity,
    Wind,
    Pressure,
    Clouds,
    Visibility,
}
