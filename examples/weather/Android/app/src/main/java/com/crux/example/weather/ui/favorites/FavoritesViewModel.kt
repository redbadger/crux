package com.crux.example.weather.ui.favorites

import androidx.lifecycle.viewModelScope
import com.crux.example.weather.Event
import com.crux.example.weather.FavoritesEvent
import com.crux.example.weather.Location
import com.crux.example.weather.WeatherEvent
import com.crux.example.weather.Workflow
import com.crux.example.weather.WorkflowViewModel
import com.crux.example.weather.core.Core
import com.crux.example.weather.utils.stateInWhileSubscribed
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.map

class FavoritesViewModel(
    private val core: Core,
    private val uiStateMapper: FavoritesUiStateMapper,
) : androidx.lifecycle.ViewModel() {

    init {
        core.update(Event.Favorites(FavoritesEvent.Restore))
    }

    private val initialState = FavoritesUiState(
        favorites = emptyList(),
        deleteConfirmation = null,
    )

    val state: StateFlow<FavoritesUiState> = core.workflowViewModel<WorkflowViewModel.Favorites>()
        .map { uiStateMapper.map(it) }
        .stateInWhileSubscribed(
            scope = viewModelScope,
            initialValue = initialState,
        )

    fun onNavigateHome() {
        core.update(Event.Navigate(Workflow.Home))
    }

    fun onAddFavorite() {
        core.update(Event.Navigate(Workflow.AddFavorite))
    }

    fun onSelectFavorite() {
        core.update(Event.Home(WeatherEvent.SHOW))
    }

    fun onDeletePressed(location: Location) {
        core.update(Event.Favorites(FavoritesEvent.DeletePressed(location)))
    }

    fun onDeleteConfirmed() {
        core.update(Event.Favorites(FavoritesEvent.DeleteConfirmed))
    }

    fun onDeleteCancelled() {
        core.update(Event.Favorites(FavoritesEvent.DeleteCancelled))
    }
}
