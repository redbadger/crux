package com.crux.example.weather.ui.home

import androidx.lifecycle.viewModelScope
import com.crux.example.weather.Event
import com.crux.example.weather.FavoritesState
import com.crux.example.weather.WeatherEvent
import com.crux.example.weather.Workflow
import com.crux.example.weather.WorkflowViewModel
import com.crux.example.weather.core.Core
import com.crux.example.weather.utils.stateInWhileSubscribed
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.map

class HomeViewModel(
    private val core: Core,
    private val uiStateMapper: HomeUiStateMapper,
) : androidx.lifecycle.ViewModel() {

    init {
        core.update(Event.Home(WeatherEvent.SHOW))
    }

    private val initialState = HomeUiState(pages = listOf(HomePageUi.Loading))
    val state: StateFlow<HomeUiState> = core.workflowViewModel<WorkflowViewModel.Home>()
        .map { uiStateMapper.map(it) }
        .stateInWhileSubscribed(
            scope = viewModelScope,
            initialValue = initialState,
        )

    fun onShowFavorites() {
        core.update(Event.Navigate(Workflow.Favorites(FavoritesState.Idle)))
    }
}