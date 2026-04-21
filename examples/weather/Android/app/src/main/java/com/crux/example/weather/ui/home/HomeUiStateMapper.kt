package com.crux.example.weather.ui.home

import com.crux.example.weather.CurrentWeatherResponse
import com.crux.example.weather.FavoriteWeatherStateViewModel
import com.crux.example.weather.FavoriteWeatherViewModel
import com.crux.example.weather.HomeViewModel
import com.crux.example.weather.LocalWeatherViewModel
import java.time.Instant
import java.util.Locale
import javax.inject.Inject

class HomeUiStateMapper @Inject constructor() {

    fun map(home: HomeViewModel): HomeUiState {
        return HomeUiState(pages = mapPages(home))
    }

    private fun mapPages(home: HomeViewModel): List<HomePageUi> {
        val pages = ArrayList<HomePageUi>(1 + home.favorites.size)
        pages.add(mapLocalWeather(home.localWeather))

        for (favorite in home.favorites) {
            pages.add(mapFavoriteWeather(favorite))
        }

        return pages
    }

    private fun mapLocalWeather(local: LocalWeatherViewModel): HomePageUi {
        return when (local) {
            is LocalWeatherViewModel.CheckingPermission,
            is LocalWeatherViewModel.FetchingLocation,
            is LocalWeatherViewModel.FetchingWeather -> HomePageUi.Loading

            is LocalWeatherViewModel.LocationDisabled -> HomePageUi.LocationDisabled
            is LocalWeatherViewModel.Fetched -> toPage(local.value)
            is LocalWeatherViewModel.Failed -> HomePageUi.Error
        }
    }

    private fun mapFavoriteWeather(favorite: FavoriteWeatherViewModel): HomePageUi {
        return when (val weather = favorite.weather) {
            is FavoriteWeatherStateViewModel.Fetching -> HomePageUi.Loading
            is FavoriteWeatherStateViewModel.Fetched -> toPage(weather.value, favorite.name)
            is FavoriteWeatherStateViewModel.Failed -> HomePageUi.Error
        }
    }

    private fun toPage(
        weatherData: CurrentWeatherResponse,
        nameOverride: String? = null,
    ): HomePageUi {
        if (!isValidWeather(weatherData)) {
            return HomePageUi.Loading
        }
        return HomePageUi.Weather(toCardUi(weatherData, nameOverride))
    }

    private fun isValidWeather(weatherData: CurrentWeatherResponse): Boolean {
        return weatherData.cod == 200UL && weatherData.main.temp.isFinite()
    }

    private fun toCardUi(
        weatherData: CurrentWeatherResponse,
        nameOverride: String? = null,
    ): WeatherCardUi {
        val isDay = isDaytime(weatherData)
        val weather = weatherData.weather.firstOrNull()
        val condition = weather?.main
        val description = weather?.description?.replaceFirstChar { ch ->
            if (ch.isLowerCase()) ch.titlecase(Locale.US) else ch.toString()
        }
        return WeatherCardUi(
            title = nameOverride ?: weatherData.name,
            temperature = weatherData.main.temp,
            condition = condition,
            description = description,
            tempMin = weatherData.main.tempMin,
            tempMax = weatherData.main.tempMax,
            details = weatherDetails(weatherData),
            sunrise = weatherData.sys.sunrise.toLong(),
            sunset = weatherData.sys.sunset.toLong(),
            weatherType = weather?.let { weatherTypeForCode(it.id, isDay) }
                ?: WeatherType.Unknown,
        )
    }

    private fun isDaytime(weatherData: CurrentWeatherResponse): Boolean {
        val now = Instant.now().epochSecond
        return now >= weatherData.sys.sunrise.toLong() && now <= weatherData.sys.sunset.toLong()
    }

    private fun weatherTypeForCode(
        weatherCode: ULong,
        isDay: Boolean,
    ): WeatherType {
        return when (weatherCode.toLong()) {
            in 200..232 -> WeatherType.Thunderstorm
            in 300..321 -> WeatherType.Drizzle
            in 500..531 -> WeatherType.Rain
            in 600..622 -> WeatherType.Snow
            in 701..781 -> WeatherType.Atmosphere
            800L -> if (isDay) WeatherType.ClearDay else WeatherType.ClearNight
            in 801..804 -> WeatherType.Clouds
            else -> WeatherType.Unknown
        }
    }

    private fun weatherDetails(weatherData: CurrentWeatherResponse): List<WeatherDetailUi> {
        return listOf(
            WeatherDetailUi(
                type = WeatherDetailType.FeelsLike,
                value = weatherData.main.feelsLike,
            ),
            WeatherDetailUi(
                type = WeatherDetailType.Humidity,
                value = weatherData.main.humidity.toDouble(),
            ),
            WeatherDetailUi(
                type = WeatherDetailType.Wind,
                value = weatherData.wind.speed,
            ),
            WeatherDetailUi(
                type = WeatherDetailType.Pressure,
                value = weatherData.main.pressure.toDouble(),
            ),
            WeatherDetailUi(
                type = WeatherDetailType.Clouds,
                value = weatherData.clouds.all.toDouble(),
            ),
            WeatherDetailUi(
                type = WeatherDetailType.Visibility,
                value = weatherData.visibility.toDouble(),
            ),
        )
    }
}
