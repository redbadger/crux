package com.crux.example.weather.ui.favorites

import androidx.compose.runtime.Immutable
import com.crux.example.weather.GeocodingResponse
import com.crux.example.weather.Location

@Immutable
data class FavoritesUiState(
    val favorites: List<FavoriteItemUi>,
    val workflow: FavoritesWorkflowUiState? = null,
)

@Immutable
data class FavoriteItemUi(
    val name: String,
    val location: Location,
)

sealed interface FavoritesWorkflowUiState {
    data class ConfirmDelete(val location: Location) : FavoritesWorkflowUiState

    data class Add(
        val searchInput: String,
        val searchResults: List<GeocodingResponse>?,
        val searching: Boolean,
    ) : FavoritesWorkflowUiState
}
