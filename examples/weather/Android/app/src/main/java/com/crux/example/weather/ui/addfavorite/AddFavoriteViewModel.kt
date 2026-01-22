package com.crux.example.weather.ui.addfavorite

import androidx.lifecycle.viewModelScope
import com.crux.example.weather.Event
import com.crux.example.weather.FavoritesEvent
import com.crux.example.weather.GeocodingResponse
import com.crux.example.weather.WorkflowViewModel
import com.crux.example.weather.core.Core
import com.crux.example.weather.utils.stateInWhileSubscribed
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.map

class AddFavoriteViewModel(
    private val core: Core,
    private val uiStateMapper: AddFavoriteUiStateMapper,
) : androidx.lifecycle.ViewModel() {

    private val initialState = AddFavoriteUiState(searchResults = null)

    val state: StateFlow<AddFavoriteUiState> =
        core.workflowViewModel<WorkflowViewModel.AddFavorite>()
            .map { uiStateMapper.map(it) }
            .stateInWhileSubscribed(
                scope = viewModelScope,
                initialValue = initialState,
            )

    fun onSearch(query: String) {
        core.update(Event.Favorites(FavoritesEvent.Search(query)))
    }

    fun onSubmit(result: GeocodingResponse) {
        core.update(Event.Favorites(FavoritesEvent.Submit(result)))
    }

    fun onCancel() {
        core.update(Event.Favorites(FavoritesEvent.Cancel))
    }
}
