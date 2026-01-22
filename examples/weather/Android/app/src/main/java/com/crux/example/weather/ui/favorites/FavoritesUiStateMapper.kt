package com.crux.example.weather.ui.favorites

import com.crux.example.weather.WorkflowViewModel

class FavoritesUiStateMapper {
    fun map(favorites: WorkflowViewModel.Favorites): FavoritesUiState {
        return FavoritesUiState(
            favorites = favorites.favorites.map {
                FavoriteItemUi(
                    name = it.name,
                    location = it.location,
                )
            },
            deleteConfirmation = favorites.deleteConfirmation,
        )
    }
}
