package com.crux.example.weather.ui.addfavorite

import androidx.compose.runtime.Immutable
import com.crux.example.weather.GeocodingResponse

@Immutable
data class AddFavoriteUiState(
    val searchResults: List<GeocodingResponse>?,
)
