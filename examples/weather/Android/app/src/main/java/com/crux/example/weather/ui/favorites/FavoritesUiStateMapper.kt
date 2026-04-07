package com.crux.example.weather.ui.favorites

import com.crux.example.weather.FavoritesViewModel
import com.crux.example.weather.FavoritesWorkflowViewModel

class FavoritesUiStateMapper {
    fun map(favorites: FavoritesViewModel): FavoritesUiState {
        return FavoritesUiState(
            favorites = favorites.favorites.map {
                FavoriteItemUi(
                    name = it.name,
                    location = it.location,
                )
            },
            workflow = favorites.workflow?.let { mapWorkflow(it) },
        )
    }

    private fun mapWorkflow(
        workflow: FavoritesWorkflowViewModel,
    ): FavoritesWorkflowUiState {
        return when (workflow) {
            is FavoritesWorkflowViewModel.ConfirmDelete -> {
                FavoritesWorkflowUiState.ConfirmDelete(workflow.location)
            }

            is FavoritesWorkflowViewModel.Add -> {
                FavoritesWorkflowUiState.Add(
                    searchInput = workflow.value.searchInput,
                    searchResults = workflow.value.searchResults,
                    searching = workflow.value.searching,
                )
            }
        }
    }
}
