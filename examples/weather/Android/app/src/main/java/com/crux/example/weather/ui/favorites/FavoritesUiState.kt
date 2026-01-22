package com.crux.example.weather.ui.favorites

import androidx.compose.runtime.Immutable
import com.crux.example.weather.Location

@Immutable
data class FavoritesUiState(
    val favorites: List<FavoriteItemUi>,
    val deleteConfirmation: Location?,
)

@Immutable
data class FavoriteItemUi(
    val name: String,
    val location: Location,
)
