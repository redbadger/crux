package com.crux.example.weather.ui.addfavorite

import com.crux.example.weather.WorkflowViewModel

class AddFavoriteUiStateMapper {
    fun map(addFavorite: WorkflowViewModel.AddFavorite): AddFavoriteUiState {
        return AddFavoriteUiState(
            searchResults = addFavorite.searchResults,
        )
    }
}
