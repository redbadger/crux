package com.crux.example.weather.ui.home

import androidx.lifecycle.viewModelScope
import com.crux.example.weather.ActiveEvent
import com.crux.example.weather.Event
import com.crux.example.weather.HomeEvent
import com.crux.example.weather.core.Core
import com.crux.example.weather.utils.stateInWhileSubscribed
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.map

class HomeViewModel(
    private val core: Core,
    private val uiStateMapper: HomeUiStateMapper,
) : androidx.lifecycle.ViewModel() {

    private val initialState = HomeUiState(pages = listOf(HomePageUi.Loading))
    val state: StateFlow<HomeUiState> = core.homeViewModel()
        .map { uiStateMapper.map(it) }
        .stateInWhileSubscribed(
            scope = viewModelScope,
            initialValue = initialState,
        )

    fun onShowFavorites() {
        core.update(Event.Active(ActiveEvent.Home(HomeEvent.GOTOFAVORITES)))
    }

    fun onResetApiKey() {
        core.update(Event.Active(ActiveEvent.ResetApiKey))
    }
}
