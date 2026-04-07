package com.crux.example.weather.ui.favorites

import androidx.lifecycle.viewModelScope
import com.crux.example.weather.ActiveEvent
import com.crux.example.weather.AddFavoriteEvent
import com.crux.example.weather.ConfirmDeleteEvent
import com.crux.example.weather.Event
import com.crux.example.weather.FavoritesScreenEvent
import com.crux.example.weather.FavoritesWorkflowEvent
import com.crux.example.weather.GeocodingResponse
import com.crux.example.weather.Location
import com.crux.example.weather.core.Core
import com.crux.example.weather.utils.stateInWhileSubscribed
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.map

class FavoritesViewModel(
    private val core: Core,
    private val uiStateMapper: FavoritesUiStateMapper,
) : androidx.lifecycle.ViewModel() {

    private val initialState = FavoritesUiState(favorites = emptyList())

    val state: StateFlow<FavoritesUiState> = core.favoritesViewModel()
        .map { uiStateMapper.map(it) }
        .stateInWhileSubscribed(
            scope = viewModelScope,
            initialValue = initialState,
        )

    fun onNavigateHome() {
        core.update(
            Event.Active(ActiveEvent.Favorites(FavoritesScreenEvent.GoToHome))
        )
    }

    fun onAddFavorite() {
        core.update(
            Event.Active(ActiveEvent.Favorites(FavoritesScreenEvent.RequestAddFavorite))
        )
    }

    fun onDeletePressed(location: Location) {
        core.update(
            Event.Active(ActiveEvent.Favorites(FavoritesScreenEvent.RequestDelete(location)))
        )
    }

    fun onDeleteConfirmed() {
        core.update(
            Event.Active(
                ActiveEvent.Favorites(
                    FavoritesScreenEvent.Workflow(
                        FavoritesWorkflowEvent.ConfirmDelete(ConfirmDeleteEvent.CONFIRMED)
                    )
                )
            )
        )
    }

    fun onDeleteCancelled() {
        core.update(
            Event.Active(
                ActiveEvent.Favorites(
                    FavoritesScreenEvent.Workflow(
                        FavoritesWorkflowEvent.ConfirmDelete(ConfirmDeleteEvent.CANCELLED)
                    )
                )
            )
        )
    }

    fun onSearch(query: String) {
        core.update(
            Event.Active(
                ActiveEvent.Favorites(
                    FavoritesScreenEvent.Workflow(
                        FavoritesWorkflowEvent.Add(AddFavoriteEvent.Search(query))
                    )
                )
            )
        )
    }

    fun onSubmitFavorite(geocodingResponse: GeocodingResponse) {
        core.update(
            Event.Active(
                ActiveEvent.Favorites(
                    FavoritesScreenEvent.Workflow(
                        FavoritesWorkflowEvent.Add(AddFavoriteEvent.Submit(geocodingResponse))
                    )
                )
            )
        )
    }

    fun onCancelAdd() {
        core.update(
            Event.Active(
                ActiveEvent.Favorites(
                    FavoritesScreenEvent.Workflow(
                        FavoritesWorkflowEvent.Add(AddFavoriteEvent.Cancel)
                    )
                )
            )
        )
    }
}
