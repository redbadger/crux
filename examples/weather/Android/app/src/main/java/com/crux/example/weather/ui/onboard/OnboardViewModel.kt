package com.crux.example.weather.ui.onboard

import androidx.lifecycle.viewModelScope
import com.crux.example.weather.Event
import com.crux.example.weather.OnboardEvent
import com.crux.example.weather.OnboardReason
import com.crux.example.weather.OnboardStateViewModel
import com.crux.example.weather.core.Core
import com.crux.example.weather.utils.stateInWhileSubscribed
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.map

class OnboardViewModel(
    private val core: Core,
) : androidx.lifecycle.ViewModel() {

    private val initialState = OnboardUiState(
        reason = OnboardReason.WELCOME,
        apiKey = "",
        canSubmit = false,
        isSaving = false,
    )

    val state: StateFlow<OnboardUiState> = core.onboardViewModel()
        .map { onboard ->
            when (val s = onboard.state) {
                is OnboardStateViewModel.Input -> OnboardUiState(
                    reason = onboard.reason,
                    apiKey = s.apiKey,
                    canSubmit = s.canSubmit,
                    isSaving = false,
                )

                is OnboardStateViewModel.Saving -> OnboardUiState(
                    reason = onboard.reason,
                    apiKey = "",
                    canSubmit = false,
                    isSaving = true,
                )
            }
        }
        .stateInWhileSubscribed(
            scope = viewModelScope,
            initialValue = initialState,
        )

    fun onApiKeyChanged(text: String) {
        core.update(Event.Onboard(OnboardEvent.ApiKey(text)))
    }

    fun onSubmit() {
        core.update(Event.Onboard(OnboardEvent.Submit))
    }
}
