package com.crux.example.weather.ui.home

import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.AcUnit
import androidx.compose.material.icons.outlined.Air
import androidx.compose.material.icons.outlined.ArrowDownward
import androidx.compose.material.icons.outlined.Cloud
import androidx.compose.material.icons.outlined.DeviceThermostat
import androidx.compose.material.icons.outlined.Grain
import androidx.compose.material.icons.outlined.NightsStay
import androidx.compose.material.icons.outlined.Speed
import androidx.compose.material.icons.outlined.Star
import androidx.compose.material.icons.outlined.Thunderstorm
import androidx.compose.material.icons.outlined.Umbrella
import androidx.compose.material.icons.outlined.Visibility
import androidx.compose.material.icons.outlined.WaterDrop
import androidx.compose.material.icons.outlined.WbSunny
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.IconButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.runtime.collectAsState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.crux.example.weather.R
import com.crux.example.weather.ui.theme.WeatherTheme
import org.koin.androidx.compose.koinViewModel
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter

@Composable
fun HomeScreen() {
    val viewModel = koinViewModel<HomeViewModel>()
    HomeScreen(
        homeUiState = viewModel.state.collectAsState().value,
        onShowFavorites = viewModel::onShowFavorites,
    )
}

@OptIn(ExperimentalFoundationApi::class, ExperimentalMaterial3Api::class)
@Composable
private fun HomeScreen(
    homeUiState: HomeUiState,
    onShowFavorites: () -> Unit,
) {
    val pagerState = rememberPagerState(initialPage = 0) {
        homeUiState.pages.size
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { },
                actions = {
                    IconButton(
                        onClick = onShowFavorites,
                        shape = CircleShape,
                        modifier = Modifier.size(48.dp),
                        colors = IconButtonDefaults.iconButtonColors(
                            containerColor = MaterialTheme.colorScheme.surfaceContainer,
                        ),
                    ) {
                        Icon(
                            imageVector = Icons.Outlined.Star,
                            contentDescription = "Favorites",
                        )
                    }
                }
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .padding(padding)
                .padding(vertical = 16.dp)
                .fillMaxSize()
        ) {
            val weatherCardModifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp)
            HorizontalPager(
                state = pagerState,
                modifier = Modifier.fillMaxSize(),
            ) { page ->
                when (val pageUi = homeUiState.pages.getOrNull(page)) {
                    is HomePageUi.Weather -> WeatherCard(pageUi.card, weatherCardModifier)
                    HomePageUi.Loading, null -> LoadingCard(weatherCardModifier)
                }
            }

            if (pagerState.pageCount > 1) {
                Spacer(modifier = Modifier.height(12.dp))
                PageIndicator(
                    currentPage = pagerState.currentPage,
                    pageCount = pagerState.pageCount,
                    modifier = Modifier.fillMaxWidth()
                )
            }
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun WeatherCard(
    card: WeatherCardUi,
    modifier: Modifier = Modifier,
) {
    Card(
        modifier = modifier,
        shape = RoundedCornerShape(20.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 6.dp)
    ) {
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .background(Brush.linearGradient(weatherGradientColors(card.weatherType)))
        ) {
            Column(
                modifier = Modifier
                    .background(MaterialTheme.colorScheme.surface.copy(alpha = 0.9f))
                    .padding(24.dp)
                    .fillMaxWidth(),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Text(
                    text = card.title,
                    style = MaterialTheme.typography.headlineSmall.copy(
                        fontWeight = FontWeight.Bold,
                    ),
                    textAlign = TextAlign.Center
                )

                Spacer(modifier = Modifier.height(16.dp))

                val weatherIcon = weatherIconForType(card.weatherType)
                if (weatherIcon != null) {
                    WeatherIcon(weatherIcon)
                }
                Spacer(modifier = Modifier.height(16.dp))
                TemperatureRow(card)
                Spacer(modifier = Modifier.height(16.dp))
                MaxMinTempRow(card.tempMin, card.tempMax)
                Spacer(modifier = Modifier.height(16.dp))
                FlowRow(
                    maxItemsInEachRow = 2,
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                    verticalArrangement = Arrangement.spacedBy(16.dp),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    card.details.forEach { item ->
                        WeatherDetailItem(item, modifier = Modifier.weight(1f))
                    }
                }
                Spacer(modifier = Modifier.height(16.dp))
                SunriseSunsetRow(card.sunrise, card.sunset)
            }
        }
    }
}

@Composable
private fun LoadingCard(modifier: Modifier = Modifier) {
    Card(
        modifier = modifier,
        shape = RoundedCornerShape(20.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 4.dp)
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            CircularProgressIndicator()
            Spacer(modifier = Modifier.height(12.dp))
            Text(
                text = stringResource(R.string.weather_loading),
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}

@Composable
private fun TemperatureRow(card: WeatherCardUi) {
    Row(
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.Center
    ) {
        Text(
            text = formatTemp(card.temperature),
            style = MaterialTheme.typography.displaySmall.copy(
                fontWeight = FontWeight.Bold,
            )
        )
        Spacer(modifier = Modifier.width(16.dp))
        Column {
            if (card.condition != null) {
                Text(
                    text = card.condition,
                    style = MaterialTheme.typography.titleMedium
                )
            }
            if (card.description != null) {
                Text(
                    text = card.description,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }
    }
}

@Composable
private fun MaxMinTempRow(tempMin: Double, tempMax: Double) {
    Row(
        horizontalArrangement = Arrangement.spacedBy(16.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        MaxMinTemp(
            icon = Icons.Outlined.ArrowDownward,
            text = formatTemp(tempMin)
        )
        MaxMinTemp(
            icon = Icons.Outlined.ArrowDownward,
            text = formatTemp(tempMax)
        )
    }
}

@Composable
private fun WeatherIcon(icon: ImageVector, modifier: Modifier = Modifier) {
    Icon(
        imageVector = icon,
        contentDescription = null,
        modifier = modifier.size(40.dp),
        tint = MaterialTheme.colorScheme.primary
    )
}

private fun weatherGradientColors(type: WeatherType): List<Color> {
    return when (type) {
        WeatherType.Thunderstorm -> listOf(Color(0xFF9E9E9E), Color(0xFF5E7CE2))
        WeatherType.Drizzle -> listOf(Color(0xFF5E7CE2), Color(0xFF9E9E9E))
        WeatherType.Rain -> listOf(Color(0xFF4F8FE8), Color(0xFF90A4AE))
        WeatherType.Snow -> listOf(Color(0xFFE3F2FD), Color(0xFF64B5F6))
        WeatherType.Atmosphere -> listOf(Color(0xFF90A4AE), Color(0xFF5E7CE2))
        WeatherType.ClearDay -> listOf(Color(0xFF4F8FE8), Color(0xFFFFE082))
        WeatherType.ClearNight -> listOf(Color(0xFF4F8FE8), Color(0xFF9C27B0))
        WeatherType.Clouds -> listOf(Color(0xFF90A4AE), Color(0xFF4F8FE8))
        WeatherType.Unknown -> listOf(Color(0xFF4F8FE8), Color(0xFF90A4AE))
    }
}

private fun weatherIconForType(type: WeatherType): ImageVector? {
    return when (type) {
        WeatherType.Thunderstorm -> Icons.Outlined.Thunderstorm
        WeatherType.Drizzle -> Icons.Outlined.Grain
        WeatherType.Rain -> Icons.Outlined.Umbrella
        WeatherType.Snow -> Icons.Outlined.AcUnit
        WeatherType.Atmosphere -> Icons.Outlined.Visibility
        WeatherType.ClearDay -> Icons.Outlined.WbSunny
        WeatherType.ClearNight -> Icons.Outlined.NightsStay
        WeatherType.Clouds -> Icons.Outlined.Cloud
        WeatherType.Unknown -> Icons.Outlined.WbSunny
    }
}

private data class WeatherDetailPresentation(
    val icon: ImageVector,
    val title: String,
)

@ReadOnlyComposable
@Composable
private fun getWeatherDetailPresentation(type: WeatherDetailType): WeatherDetailPresentation {
    return when (type) {
        WeatherDetailType.FeelsLike -> WeatherDetailPresentation(
            icon = Icons.Outlined.DeviceThermostat,
            title = stringResource(R.string.weather_detail_feels_like)
        )

        WeatherDetailType.Humidity -> WeatherDetailPresentation(
            icon = Icons.Outlined.WaterDrop,
            title = stringResource(R.string.weather_detail_humidity)
        )

        WeatherDetailType.Wind -> WeatherDetailPresentation(
            icon = Icons.Outlined.Air,
            title = stringResource(R.string.weather_detail_wind)
        )

        WeatherDetailType.Pressure -> WeatherDetailPresentation(
            icon = Icons.Outlined.Speed,
            title = stringResource(R.string.weather_detail_pressure)
        )

        WeatherDetailType.Clouds -> WeatherDetailPresentation(
            icon = Icons.Outlined.Cloud,
            title = stringResource(R.string.weather_detail_clouds)
        )

        WeatherDetailType.Visibility -> WeatherDetailPresentation(
            icon = Icons.Outlined.Visibility,
            title = stringResource(R.string.weather_detail_visibility)
        )
    }
}

@ReadOnlyComposable
@Composable
private fun formatTemp(value: Double): String {
    return stringResource(R.string.weather_temp_format, value)
}

@ReadOnlyComposable
@Composable
private fun formatDetailValue(type: WeatherDetailType, value: Double): String {
    return when (type) {
        WeatherDetailType.FeelsLike -> {
            stringResource(R.string.weather_temp_format, value)
        }

        WeatherDetailType.Humidity -> stringResource(
            R.string.weather_humidity_format,
            value.toInt()
        )

        WeatherDetailType.Wind -> {
            stringResource(R.string.weather_wind_format, value)
        }

        WeatherDetailType.Pressure -> stringResource(
            R.string.weather_pressure_format,
            value.toInt()
        )

        WeatherDetailType.Clouds -> {
            stringResource(R.string.weather_clouds_format, value.toInt())
        }

        WeatherDetailType.Visibility -> {
            val km = (value / 1000.0).toInt()
            stringResource(R.string.weather_visibility_format, km)
        }
    }
}

@Composable
private fun formatTime(epochSeconds: Long): String {
    val pattern = stringResource(R.string.weather_time_format)
    val instant = Instant.ofEpochSecond(epochSeconds)
    return DateTimeFormatter.ofPattern(pattern)
        .withZone(ZoneId.systemDefault())
        .format(instant)
}

@Composable
private fun WeatherDetailItem(
    item: WeatherDetailUi,
    modifier: Modifier = Modifier,
) {
    val detailPresentation = getWeatherDetailPresentation(item.type)
    val value = formatDetailValue(item.type, item.value)
    Card(
        modifier = modifier,
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surface,
        ),
        elevation = CardDefaults.cardElevation(defaultElevation = 2.dp),
    ) {
        Row(
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp),
        ) {
            Icon(
                imageVector = detailPresentation.icon,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.primary
            )
            Column {
                Text(
                    text = detailPresentation.title,
                    style = MaterialTheme.typography.labelMedium
                )
                Text(
                    text = value,
                    style = MaterialTheme.typography.bodyMedium.copy(
                        fontWeight = FontWeight.SemiBold,
                    ),
                )
            }
        }
    }
}

@Composable
private fun SunriseSunsetRow(sunriseTimestamp: Long, sunsetTimestamp: Long) {
    Row(
        horizontalArrangement = Arrangement.spacedBy(24.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        SunriseSunsetItem(
            icon = Icons.Outlined.WbSunny,
            time = formatTime(sunriseTimestamp),
            label = stringResource(R.string.weather_sunrise)
        )
        SunriseSunsetItem(
            icon = Icons.Outlined.NightsStay,
            time = formatTime(sunsetTimestamp),
            label = stringResource(R.string.weather_sunset)
        )
    }
}

@Composable
private fun SunriseSunsetItem(icon: ImageVector, time: String, label: String) {
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        Icon(
            imageVector = icon,
            contentDescription = label,
            tint = MaterialTheme.colorScheme.primary
        )
        Spacer(modifier = Modifier.height(4.dp))
        Text(text = time, style = MaterialTheme.typography.bodyMedium)
    }
}

@Composable
private fun MaxMinTemp(icon: ImageVector, text: String) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        Icon(
            imageVector = icon,
            contentDescription = null,
            tint = MaterialTheme.colorScheme.onSurfaceVariant
        )
        Spacer(modifier = Modifier.width(4.dp))
        Text(
            text = text,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}

@Composable
private fun PageIndicator(currentPage: Int, pageCount: Int, modifier: Modifier = Modifier) {
    Row(
        modifier = modifier,
        horizontalArrangement = Arrangement.Center
    ) {
        repeat(pageCount) { index ->
            val color = if (index == currentPage) {
                MaterialTheme.colorScheme.primary
            } else {
                MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.4f)
            }
            Box(
                modifier = Modifier
                    .padding(horizontal = 4.dp)
                    .size(8.dp)
                    .clip(CircleShape)
                    .background(color)
            )
        }
    }
}

@Preview(showBackground = true)
@Composable
private fun HomeScreenPreview() {
    WeatherTheme(darkTheme = false, dynamicColor = false) {
        HomeScreen(
            homeUiState = sampleHomeUiState(),
            onShowFavorites = {}
        )
    }
}

@Preview(showBackground = true)
@Composable
private fun HomeScreenLoadingPreview() {
    WeatherTheme(darkTheme = false, dynamicColor = false) {
        HomeScreen(
            homeUiState = HomeUiState(pages = listOf(HomePageUi.Loading)),
            onShowFavorites = {}
        )
    }
}

private fun sampleHomeUiState(): HomeUiState {
    return HomeUiState(
        pages = listOf(
            HomePageUi.Weather(
                WeatherCardUi(
                    title = "San Francisco",
                    temperature = 64.0,
                    condition = "Sunny",
                    description = "Clear skies",
                    tempMin = 58.0,
                    tempMax = 70.0,
                    details = listOf(
                        WeatherDetailUi(WeatherDetailType.FeelsLike, 63.0),
                        WeatherDetailUi(WeatherDetailType.Humidity, 45.0),
                        WeatherDetailUi(WeatherDetailType.Wind, 5.2),
                        WeatherDetailUi(WeatherDetailType.Pressure, 1012.0),
                        WeatherDetailUi(WeatherDetailType.Clouds, 12.0),
                        WeatherDetailUi(WeatherDetailType.Visibility, 12000.0)
                    ),
                    sunrise = 1702207200,
                    sunset = 1702242000,
                    weatherType = WeatherType.ClearDay
                )
            ),
            HomePageUi.Weather(
                WeatherCardUi(
                    title = "Chicago",
                    temperature = 36.0,
                    condition = "Snow",
                    description = "Light flurries",
                    tempMin = 30.0,
                    tempMax = 39.0,
                    details = listOf(
                        WeatherDetailUi(WeatherDetailType.FeelsLike, 28.0),
                        WeatherDetailUi(WeatherDetailType.Humidity, 62.0),
                        WeatherDetailUi(WeatherDetailType.Wind, 10.4),
                        WeatherDetailUi(WeatherDetailType.Pressure, 1004.0),
                        WeatherDetailUi(WeatherDetailType.Clouds, 80.0),
                        WeatherDetailUi(WeatherDetailType.Visibility, 8000.0)
                    ),
                    sunrise = 1702210200,
                    sunset = 1702239000,
                    weatherType = WeatherType.Snow
                )
            )
        )
    )
}
